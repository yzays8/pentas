use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;
use std::fmt;
use std::rc::Rc;

use anyhow::{Context, Ok, Result};

use crate::renderer::css::cssom::{ComponentValue, Declaration, Rule, StyleSheet};
use crate::renderer::css::dtype::{AbsoluteSize, CssValue};
use crate::renderer::css::property::border::{parse_border, BorderProp};
use crate::renderer::css::property::border::{BorderStyleProp, BorderWidthProp};
use crate::renderer::css::property::color::{parse_color, ColorProp};
use crate::renderer::css::property::display::{
    parse_display, DisplayBox, DisplayInside, DisplayOutside, DisplayProp,
};
use crate::renderer::css::property::font_size::{parse_font_size, FontSizeProp};
use crate::renderer::css::property::height::{parse_height, HeightProp};
use crate::renderer::css::property::margin::{
    parse_margin, parse_margin_block, MarginBlockProp, MarginProp,
};
use crate::renderer::css::property::padding::{parse_padding, PaddingProp};
use crate::renderer::css::property::text_decoration::{parse_text_decoration, TextDecorationProp};
use crate::renderer::css::property::width::{parse_width, WidthProp};
use crate::renderer::css::selector::Selector;
use crate::renderer::html::dom::{DocumentTree, DomNode, Element, NodeType};
use crate::renderer::layout::BoxTree;

#[derive(Debug)]
pub struct RenderTree {
    pub root: Rc<RefCell<RenderNode>>,
}

impl RenderTree {
    pub fn build(document_tree: &DocumentTree, style_sheets: Vec<StyleSheet>) -> Result<Self> {
        Ok(Self {
            root: Rc::new(RefCell::new(
                RenderNode::build(Rc::clone(&document_tree.root), &style_sheets, None)?
                    .context("Failed to build the render tree.")?,
            )),
        })
    }

    pub fn print(&self) {
        println!("{}", self);
    }

    pub fn to_box_tree(&self) -> Result<BoxTree> {
        BoxTree::build(self)
    }
}

impl fmt::Display for RenderTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn construct_node_view(
            node_tree: &mut String,
            node: &Rc<RefCell<RenderNode>>,
            current_depth: usize,
            is_last_child: bool,
            mut exclude_branches: Vec<usize>,
        ) {
            if is_last_child {
                exclude_branches.push(current_depth);
            }
            let mut indent_and_branches = String::new();
            for i in 0..current_depth {
                if exclude_branches.contains(&i) {
                    indent_and_branches.push_str("  ");
                } else {
                    indent_and_branches.push_str("│ ");
                }
            }
            indent_and_branches.push_str(if is_last_child { "└─" } else { "├─" });
            node_tree.push_str(&format!("{}{}\n", indent_and_branches, node.borrow()));
            let children_num = node.borrow().child_nodes.len();
            for (i, child) in node.borrow().child_nodes.iter().enumerate() {
                construct_node_view(
                    node_tree,
                    child,
                    current_depth + 1,
                    i == children_num - 1,
                    exclude_branches.clone(),
                );
            }
        }
        let mut node_tree = String::new();
        construct_node_view(&mut node_tree, &self.root, 0, true, vec![]);
        node_tree.pop(); // Remove the last newline character
        write!(f, "{}", node_tree)
    }
}

#[derive(Debug)]
pub struct RenderNode {
    pub node: Rc<RefCell<DomNode>>,
    pub style: ComputedValues,
    pub child_nodes: Vec<Rc<RefCell<Self>>>,
}

impl RenderNode {
    pub fn build(
        node: Rc<RefCell<DomNode>>,
        style_sheets: &Vec<StyleSheet>,
        parent_style: Option<ComputedValues>,
    ) -> Result<Option<Self>> {
        // Omit nodes that are not rendered.
        match &node.borrow().node_type {
            NodeType::Element(Element { tag_name, .. }) => {
                if let "script" | "meta" = tag_name.as_str() {
                    return Ok(None);
                }
            }
            NodeType::Comment(_) => {
                return Ok(None);
            }
            _ => {}
        }

        let style = if let NodeType::Element(_) = &node.borrow().node_type {
            // https://www.w3.org/TR/css-cascade-3/#value-stages
            apply_filtering(Rc::clone(&node), style_sheets)
                .apply_cascading()
                .apply_defaulting(&parent_style)?
                .apply_computing()
        } else {
            ComputedValues::default()
        };

        // All elements with a value of none for the display property and their descendants are not rendered.
        // https://developer.mozilla.org/en-US/docs/Web/CSS/display#none
        if style.display.is_some()
            && (style.display.as_ref().unwrap().display_box == Some(DisplayBox::None))
        {
            return Ok(None);
        }

        let child_nodes = node
            .borrow()
            .child_nodes
            .iter()
            .map(|child| Self::build(Rc::clone(child), style_sheets, Some(style.clone())))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            // Skip the children that are not rendered.
            .filter(|child| child.is_some())
            .map(|child| Rc::new(RefCell::new(child.unwrap())))
            .collect::<Vec<_>>();

        Ok(Some(Self {
            node: Rc::clone(&node),
            style,
            child_nodes,
        }))
    }

    pub fn get_display_type(&self) -> DisplayOutside {
        if self.style.display.is_none() {
            // The default value of the `display` property is inline.
            // The text sequence is treated as a single inline type here.
            return DisplayOutside::Inline;
        }
        self.style.display.as_ref().unwrap().outside
    }
}

impl fmt::Display for RenderNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let NodeType::Element(elm) = &self.node.borrow().node_type {
            write!(f, "{}, Computed( {})", elm, self.style)
        } else {
            write!(f, "{:?}", self.node.borrow().node_type)
        }
    }
}

/// Returns all declared values that match the node.
/// https://www.w3.org/TR/css-cascade-3/#filtering
fn apply_filtering(node: Rc<RefCell<DomNode>>, style_sheets: &[StyleSheet]) -> DeclaredValues {
    let mut declared_values = Vec::new();

    // As for the order of appearance in the subsequent cascading stage, the declarations from style sheets independently
    // linked by the originating document are treated as if they were concatenated in linking order, as determined by the host document language.
    style_sheets.iter().for_each(|style_sheet| {
        style_sheet.rules.iter().for_each(|rule| {
            let selectors = rule.get_matched_selectors(Rc::clone(&node));
            if selectors.is_some() {
                let Rule::QualifiedRule(qualified_rule) = rule;
                for selector in selectors.unwrap() {
                    declared_values.push((selector, qualified_rule.declarations.clone()));
                }
            }
        });
    });

    DeclaredValues::new(declared_values)
}

#[derive(Debug)]
pub struct DeclaredValues {
    pub values: Vec<(Selector, Vec<Declaration>)>,
}

impl DeclaredValues {
    pub fn new(values: Vec<(Selector, Vec<Declaration>)>) -> Self {
        Self { values }
    }

    /// Returns the cascaded values, which are the declared values that "win" the cascade.
    /// There is at most one cascaded value per property per element.
    /// https://www.w3.org/TR/css-cascade-3/#cascading
    pub fn apply_cascading(&self) -> CascadedValues {
        // Vec<(index, declaration, specificity)>
        let mut sorted_list = self
            .values
            .iter()
            .enumerate()
            // This function assumes that the element with the lower index is the one that appears earlier in the stylesheets.
            .map(|(index, (selector, declarations))| {
                (index, declarations.clone(), selector.calc_specificity())
            })
            .collect::<Vec<_>>();

        // Sort by specificity and then by index in descending order. If the specificity is the same,
        // the order of the declarations in the stylesheet is preserved (the last declared style gets precedence).
        sorted_list.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.0.cmp(&a.0)));

        // Determine the winning (highest-priority) declarations.
        let mut cascaded_values = HashMap::new();
        for declarations in sorted_list.iter().map(|(_, declarations, _)| declarations) {
            for declaration in declarations {
                // The higher-priority declarations are placed first in the hash table,
                // and declarations placed later in the table that have lower-priority
                // with the same name are ignored.
                cascaded_values
                    .entry(declaration.name.clone())
                    .or_insert_with(|| declaration.value.clone());
            }
        }

        CascadedValues::new(cascaded_values)
    }
}

#[derive(Debug)]
pub struct CascadedValues {
    // todo: Use IndexMap to preserve the order of insertion.
    pub values: HashMap<String, Vec<ComponentValue>>,
}

impl CascadedValues {
    pub fn new(values: HashMap<String, Vec<ComponentValue>>) -> Self {
        Self { values }
    }

    /// Returns the specified values. All properties are set to their initial values or inherited values.
    /// https://www.w3.org/TR/css-cascade-3/#defaulting
    pub fn apply_defaulting(
        &self,
        parent_style: &Option<ComputedValues>,
    ) -> Result<SpecifiedValues> {
        let mut specified_values = SpecifiedValues::new();

        specified_values.initialize();

        if parent_style.is_some() {
            specified_values.inherit(parent_style.as_ref().unwrap());
        }

        specified_values.set_from(self);

        Ok(specified_values)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpecifiedValues {
    pub background_color: Option<ColorProp>,
    pub color: Option<ColorProp>,
    pub display: Option<DisplayProp>,
    pub font_size: Option<FontSizeProp>,
    pub text_decoration: Option<TextDecorationProp>,
    pub margin: Option<MarginProp>,
    pub margin_block: Option<MarginBlockProp>,
    pub border: Option<BorderProp>,
    pub padding: Option<PaddingProp>,
    pub width: Option<WidthProp>,
    pub height: Option<HeightProp>,
}

impl SpecifiedValues {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the initial values for the properties.
    pub fn initialize(&mut self) {
        // todo: Add more properties and set correct initial values
        self.background_color = None;
        self.color = Some(ColorProp::default());
        self.display = Some(DisplayProp {
            inside: DisplayInside::Flow,
            outside: DisplayOutside::Inline,
            display_box: None,
        });
        self.font_size = Some(FontSizeProp {
            size: CssValue::AbsoluteSize(AbsoluteSize::Medium),
        });
        self.text_decoration = None;
        self.margin = Some(MarginProp {
            top: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
            right: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
            bottom: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
            left: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
        });
        self.margin_block = None;
        self.border = Some(BorderProp {
            border_color: ColorProp {
                value: CssValue::Ident("currentColor".to_string()),
            },
            border_style: BorderStyleProp {
                top: CssValue::Ident("none".to_string()),
                right: CssValue::Ident("none".to_string()),
                bottom: CssValue::Ident("none".to_string()),
                left: CssValue::Ident("none".to_string()),
            },
            border_width: BorderWidthProp {
                top: CssValue::Ident("medium".to_string()),
                right: CssValue::Ident("medium".to_string()),
                bottom: CssValue::Ident("medium".to_string()),
                left: CssValue::Ident("medium".to_string()),
            },
        });
        self.padding = Some(PaddingProp {
            top: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
            right: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
            bottom: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
            left: CssValue::Length(
                0.0,
                crate::renderer::css::dtype::LengthUnit::AbsoluteLengthUnit(
                    crate::renderer::css::dtype::AbsoluteLengthUnit::Px,
                ),
            ),
        });
        self.width = Some(WidthProp {
            size: CssValue::Ident("auto".to_string()),
        });
        self.height = Some(HeightProp {
            size: CssValue::Ident("auto".to_string()),
        });
    }

    /// Sets the inherited values for all "inherited properties".
    /// The values inherited from the parent element must be the computed values.
    pub fn inherit(&mut self, parent_values: &ComputedValues) {
        if parent_values.color.is_some() {
            self.color = parent_values.color.clone();
        }
        if parent_values.font_size.is_some() {
            self.font_size = parent_values.font_size.clone();
        }
    }

    // todo: Need to keep the order of the declarations of the properties. HashMap does not guarantee the order.
    // Assumes that the computed values have been initialized and inherited.
    pub fn set_from(&mut self, cascaded_values: &CascadedValues) {
        for (name, values) in &cascaded_values.values {
            match name.as_str() {
                // https://developer.mozilla.org/en-US/docs/Web/CSS/background-color
                "background-color" => self.background_color = parse_color(values).ok(),

                // https://developer.mozilla.org/en-US/docs/Web/CSS/color
                "color" => self.color = parse_color(values).ok(),

                // https://drafts.csswg.org/css-display/#the-display-properties
                "display" => {
                    self.display = parse_display(values).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/font-size
                "font-size" => {
                    self.font_size = parse_font_size(values).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration
                "text-decoration" => {
                    self.text_decoration = parse_text_decoration(values).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/margin
                "margin" => self.margin = parse_margin(values).ok(),

                // https://developer.mozilla.org/en-US/docs/Web/CSS/margin-block
                "margin-block" => {
                    // Assume that the margin-block-start and margin-block-end values
                    // are the same as the margin-top and margin-bottom values.
                    // todo: Handle the direction of the text
                    self.margin_block = parse_margin_block(values).ok();
                    if self.margin_block.is_some() {
                        if self.margin.is_some() {
                            self.margin.as_mut().unwrap().top =
                                self.margin_block.as_ref().unwrap().start.clone();
                            self.margin.as_mut().unwrap().bottom =
                                self.margin_block.as_ref().unwrap().end.clone();
                        } else {
                            self.margin = Some(MarginProp {
                                top: self.margin_block.as_ref().unwrap().start.clone(),
                                bottom: self.margin_block.as_ref().unwrap().end.clone(),
                                ..Default::default()
                            })
                        }
                    }
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/border
                "border" => {
                    self.border = parse_border(values).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/padding
                "padding" => {
                    self.padding = parse_padding(values).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/width
                "width" => {
                    self.width = parse_width(values).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/height
                "height" => {
                    self.height = parse_height(values).ok();
                }

                _ => {}
            }
        }
    }

    /// Converts the relative values to absolute values.
    /// https://www.w3.org/TR/css-cascade-3/#computed
    pub fn apply_computing(&self) -> ComputedValues {
        let parent_font_size_px = self.font_size.clone();
        let parent_color = self.color.clone();

        let mut t = self.clone();

        // The `color` value needs to be computed earlier because it is used to calculate other properties.
        t.color
            .as_mut()
            .map(|v| v.compute(parent_color.as_ref()).ok().cloned());
        // The `font-size` value needs to be computed earlier because it is used to calculate other properties.
        t.font_size
            .as_mut()
            .map(|v| v.compute(parent_font_size_px.as_ref()).ok().cloned());
        // The `display` value needs to be computed earlier because it is used to calculate other properties.
        t.display.as_mut().map(|v| v.compute().ok().cloned());

        t.background_color
            .as_mut()
            .map(|v| v.compute(t.color.as_ref()).ok().cloned());
        t.text_decoration
            .as_mut()
            .map(|v| v.compute(t.color.as_ref()).ok().cloned());
        t.margin
            .as_mut()
            .map(|v| v.compute(t.font_size.as_ref()).ok().cloned());
        t.margin_block
            .as_mut()
            .map(|v| v.compute(t.font_size.as_ref()).ok().cloned());
        t.border.as_mut().map(|v| {
            v.compute(t.font_size.as_ref(), t.color.as_ref())
                .ok()
                .cloned()
        });
        t.padding
            .as_mut()
            .map(|v| v.compute(t.font_size.as_ref()).ok().cloned());
        t.width
            .as_mut()
            .map(|v| v.compute(t.font_size.as_ref()).ok().cloned());
        t.height
            .as_mut()
            .map(|v| v.compute(t.font_size.as_ref()).ok().cloned());

        ComputedValues {
            background_color: t.background_color,
            color: t.color,
            display: t.display,
            font_size: t.font_size,
            text_decoration: t.text_decoration,
            margin: t.margin,
            margin_block: t.margin_block,
            border: t.border,
            padding: t.padding,
            width: t.width,
            height: t.height,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct ComputedValues {
    pub background_color: Option<ColorProp>,
    pub color: Option<ColorProp>,
    pub display: Option<DisplayProp>,
    pub font_size: Option<FontSizeProp>,
    pub text_decoration: Option<TextDecorationProp>,
    pub margin: Option<MarginProp>,
    pub margin_block: Option<MarginBlockProp>,
    pub border: Option<BorderProp>,
    pub padding: Option<PaddingProp>,
    pub width: Option<WidthProp>,
    pub height: Option<HeightProp>,
}

impl fmt::Display for ComputedValues {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut style_str = String::new();
        if let Some(background_color) = &self.background_color {
            style_str.push_str(&format!("background-color: {}; ", background_color));
        }
        if let Some(color) = &self.color {
            style_str.push_str(&format!("color: {}; ", color));
        }
        if let Some(display) = &self.display {
            style_str.push_str(&format!("display: {}; ", display));
        }
        if let Some(font_size) = &self.font_size {
            style_str.push_str(&format!("font-size: {}; ", font_size));
        }
        if let Some(text_decoration) = &self.text_decoration {
            style_str.push_str(&format!("text-decoration: {}; ", text_decoration));
        }
        if let Some(margin) = &self.margin {
            style_str.push_str(&format!("margin: {}; ", margin));
        }
        // if let Some(margin_block) = &self.margin_block {
        //     style_str.push_str(&format!("margin-block: {}; ", margin_block));
        // }
        // if let Some(border) = &self.border {
        //     style_str.push_str(&format!("border: {}; ", border));
        // }
        // if let Some(padding) = &self.padding {
        //     style_str.push_str(&format!("padding: {}; ", padding));
        // }
        // if let Some(width) = &self.width {
        //     style_str.push_str(&format!("width: {}; ", width));
        // }
        // if let Some(height) = &self.height {
        //     style_str.push_str(&format!("height: {}; ", height));
        // }
        write!(f, "{}", style_str)
    }
}
