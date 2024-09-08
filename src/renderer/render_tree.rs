use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;
use std::fmt;
use std::rc::Rc;

use anyhow::{Context, Ok, Result};

use crate::renderer::css::cssom::{ComponentValue, Declaration, Rule, StyleSheet};
use crate::renderer::css::selector::Selector;
use crate::renderer::css::tokenizer::CssToken;
use crate::renderer::html::dom::{DocumentTree, DomNode, Element, NodeType};
use crate::renderer::layout::BoxTree;
use crate::renderer::{
    border::Border, color::Color, font_size::FontSizePx, margin::Margin, padding::Padding,
    text_decoration::TextDecoration,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayType {
    Inline,
    Block,
    None,
}

impl DisplayType {
    pub fn parse(values: &[ComponentValue]) -> Self {
        if let Some(ComponentValue::PreservedToken(CssToken::Ident(keyword))) = values.first() {
            match keyword.as_str() {
                "inline" => DisplayType::Inline,
                "block" => DisplayType::Block,
                "none" => DisplayType::None,
                _ => unimplemented!(),
            }
        } else {
            unimplemented!()
        }
    }
}

#[derive(Debug)]
pub struct RenderNode {
    pub node: Rc<RefCell<DomNode>>,
    pub style: ComputedValues,
    pub child_nodes: Vec<Rc<RefCell<RenderNode>>>,
}

impl RenderNode {
    pub fn build(
        node: Rc<RefCell<DomNode>>,
        style_sheets: &Vec<StyleSheet>,
        parent_style: Option<SpecifiedValues>,
        parent_font_size_px_computed: Option<FontSizePx>,
        parent_color_computed: Option<Color>,
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
            // How the values are converted:
            // https://www.w3.org/TR/css-cascade-3/#stages-examples
            filter(Rc::clone(&node), style_sheets)
                .cascade()
                .default(parent_style)
        } else {
            SpecifiedValues::new(HashMap::new())
        };

        // All elements with a value of none for the display property and their descendants are not rendered.
        // https://developer.mozilla.org/en-US/docs/Web/CSS/display#none
        if style.values.get("display").is_some_and(|value| {
            value.contains(&ComponentValue::PreservedToken(CssToken::Ident(
                "none".to_string(),
            )))
        }) {
            return Ok(None);
        }

        let computed_values =
            style.compute(parent_font_size_px_computed, parent_color_computed.as_ref())?;
        let parent_font_size_px_computed = computed_values.font_size;
        let parent_color_computed = computed_values.color;

        let child_nodes = node
            .borrow()
            .child_nodes
            .iter()
            .map(|child| {
                Self::build(
                    Rc::clone(child),
                    style_sheets,
                    Some(style.clone()),
                    parent_font_size_px_computed,
                    parent_color_computed,
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            // Skip the children that are not rendered
            .filter(|child| child.is_some())
            .map(|child| Rc::new(RefCell::new(child.unwrap())))
            .collect::<Vec<_>>();

        Ok(Some(Self {
            node: Rc::clone(&node),
            style: computed_values,
            child_nodes,
        }))
    }

    pub fn get_display_type(&self) -> DisplayType {
        if self.style.display.is_none() {
            // The default value of the `display` property is inline.
            // The text sequence is treated as a single inline type here.
            return DisplayType::Inline;
        }
        self.style.display.as_ref().unwrap().clone()
    }
}

impl fmt::Display for RenderNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let NodeType::Element(elm) = &self.node.borrow().node_type {
            write!(f, "{:?}: Computed( {})", elm, self.style)
        } else {
            write!(f, "{:?}", self.node.borrow().node_type)
        }
    }
}

/// Returns all declared values that match the node.
/// https://www.w3.org/TR/css-cascade-3/#filtering
pub fn filter(node: Rc<RefCell<DomNode>>, style_sheets: &[StyleSheet]) -> DeclaredValues {
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

    /// Returns the declared values sorted by precedence in descending order.
    /// https://www.w3.org/TR/css-cascade-3/#cascading
    fn cascade(&self) -> CascadedValues {
        // Vec<(index, declaration, specificity)>
        let mut sorted_list = self
            .values
            .iter()
            .enumerate()
            .map(|(index, (selector, declarations))| {
                (index, declarations.clone(), selector.calc_specificity())
            })
            .collect::<Vec<_>>();

        // Sort by specificity and then by index.
        // If the specificity is the same, the order of the declarations in the stylesheet is preserved (the last declared style gets precedence).
        // Note: The higher the specificity or index, the higher the priority, so this must be sorted in descending order.
        sorted_list.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.0.cmp(&a.0)));

        CascadedValues::new(
            sorted_list
                .into_iter()
                .map(|(_, declarations, _)| declarations)
                .collect(),
        )
    }
}

#[derive(Debug)]
pub struct CascadedValues {
    pub values: Vec<Vec<Declaration>>,
}

impl CascadedValues {
    pub fn new(values: Vec<Vec<Declaration>>) -> Self {
        Self { values }
    }

    /// Returns the table of the name and value pairs for the properties.
    /// https://www.w3.org/TR/css-cascade-3/#defaulting
    fn default(&self, parent_style: Option<SpecifiedValues>) -> SpecifiedValues {
        let mut style_values = HashMap::new();

        // The higher priority declarations are placed at the beginning of the hash table,
        // and the lower priority declarations with the same name are ignored.
        for declaration in &self.values {
            for name_and_value in declaration {
                style_values
                    .entry(name_and_value.name.clone())
                    .or_insert_with(|| name_and_value.value.clone());
            }
        }

        // Inherit the parent style
        if let Some(parent_style) = parent_style {
            for (name, value) in parent_style.values {
                if is_inherited_property(name.as_str()) {
                    style_values.entry(name).or_insert(value);
                }
            }
        }

        // Set the initial values
        // https://www.w3.org/TR/CSS2/propidx.html
        // todo: Add more properties

        // style_values
        //     .entry("background-color".to_string())
        //     .or_insert(vec![ComponentValue::PreservedToken(CssToken::Ident(
        //         "transparent".to_string(),
        //     ))]);
        style_values
            .entry("color".to_string())
            .or_insert(vec![ComponentValue::PreservedToken(CssToken::Ident(
                "black".to_string(),
            ))]);
        style_values
            .entry("display".to_string())
            .or_insert(vec![ComponentValue::PreservedToken(CssToken::Ident(
                "inline".to_string(),
            ))]);
        style_values.entry("font-size".to_string()).or_insert(vec![
            ComponentValue::PreservedToken(CssToken::Ident("medium".to_string())),
        ]);

        SpecifiedValues::new(style_values)
    }
}

#[derive(Clone, Debug)]
pub struct SpecifiedValues {
    pub values: HashMap<String, Vec<ComponentValue>>,
}

impl SpecifiedValues {
    pub fn new(values: HashMap<String, Vec<ComponentValue>>) -> Self {
        Self { values }
    }

    /// https://www.w3.org/TR/css-cascade-3/#computed
    fn compute(
        &self,
        parent_font_size_px: Option<FontSizePx>,
        parent_color: Option<&Color>,
    ) -> Result<ComputedValues> {
        let mut computed_values = ComputedValues::default();

        if self.values.is_empty() {
            return Ok(computed_values);
        }
        assert!(self.values.contains_key("color"));

        // The `color` value needs to be computed earlier because it is used to calculate other properties.
        let current_color = Color::parse(self.values.get("color").unwrap(), parent_color).ok();
        computed_values.color = current_color;

        for (name, value) in &self.values {
            match name.as_str() {
                // https://developer.mozilla.org/en-US/docs/Web/CSS/background-color
                "background-color" => {
                    computed_values.background_color =
                        Color::parse(value, current_color.as_ref()).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/color
                "color" => {
                    continue;
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/font-size
                "font-size" => {
                    computed_values.font_size = FontSizePx::parse(value, parent_font_size_px).ok();
                }

                // https://drafts.csswg.org/css-display/#the-display-properties
                "display" => {
                    computed_values.display = Some(DisplayType::parse(value));
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration
                "text-decoration" => {
                    computed_values.text_decoration =
                        TextDecoration::parse(value, &current_color.unwrap()).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/margin
                "margin" => {
                    computed_values.margin = Margin::parse(value, parent_font_size_px).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/border
                "border" => {
                    computed_values.border = Border::parse(value, parent_font_size_px).ok();
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/padding
                "padding" => {
                    computed_values.padding = Padding::parse(value, parent_font_size_px).ok();
                }

                _ => {}
            }
        }

        Ok(computed_values)
    }
}

#[derive(Debug, Default)]
pub struct ComputedValues {
    pub background_color: Option<Color>,
    pub color: Option<Color>,
    pub display: Option<DisplayType>,
    pub font_size: Option<FontSizePx>,
    pub text_decoration: Option<TextDecoration>,
    pub margin: Option<Margin>,
    pub border: Option<Border>,
    pub padding: Option<Padding>,
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
            style_str.push_str(&format!("display: {:?}; ", display));
        }
        if let Some(font_size) = &self.font_size {
            style_str.push_str(&format!("font-size: {}; ", font_size.size));
        }
        if let Some(text_decoration) = &self.text_decoration {
            style_str.push_str(&format!("text-decoration: {}; ", text_decoration));
        }
        if let Some(margin) = &self.margin {
            style_str.push_str(&format!("margin: {}; ", margin));
        }
        if let Some(border) = &self.border {
            style_str.push_str(&format!("border: {}; ", border));
        }
        if let Some(padding) = &self.padding {
            style_str.push_str(&format!("padding: {}; ", padding));
        }
        write!(f, "{}", style_str)
    }
}

/// https://www.w3.org/TR/CSS2/propidx.html
fn is_inherited_property(name: &str) -> bool {
    matches!(
        name,
        "azimuth"
            | "border-collapse"
            | "border-spacing"
            | "caption-side"
            | "color"
            | "cursor"
            | "direction"
            | "elevation"
            | "empty-cells"
            | "font-family"
            | "font-size"
            | "font-style"
            | "font-variant"
            | "font-weight"
            | "font"
            | "letter-spacing"
            | "line-height"
            | "list-style-image"
            | "list-style-position"
            | "list-style-type"
            | "list-style"
            | "orphans"
            | "pitch-range"
            | "pitch"
            | "quotes"
            | "richness"
            | "speak-header"
            | "speak-numeral"
            | "speak-punctuation"
            | "speak"
            | "speech-rate"
            | "stress"
            | "text-align"
            | "text-indent"
            | "text-transform"
            | "visibility"
            | "voice-family"
            | "volume"
            | "white-space"
            | "widows"
            | "word-spacing"
            | "z-index"
    )
}

#[derive(Debug)]
pub struct RenderTree {
    pub root: Rc<RefCell<RenderNode>>,
}

impl RenderTree {
    pub fn build(document_tree: &DocumentTree, style_sheets: Vec<StyleSheet>) -> Result<Self> {
        Ok(Self {
            root: Rc::new(RefCell::new(
                RenderNode::build(
                    Rc::clone(&document_tree.root),
                    &style_sheets,
                    None,
                    None,
                    None,
                )?
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
