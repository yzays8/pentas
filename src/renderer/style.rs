pub mod property;

use std::cell::RefCell;
use std::default::Default;
use std::fmt;
use std::rc::Rc;

use anyhow::{Context, Result, anyhow};
use gtk4::pango;
use indexmap::IndexMap;

use crate::renderer::css::cssom::{ComponentValue, Declaration, Rule, StyleSheet};
use crate::renderer::css::selector::Selector;
use crate::renderer::html::dom::{DocumentTree, DomNode, NodeType};
use crate::renderer::layout::BoxTree;
use crate::renderer::style::property::{
    BackGroundColorProp, BorderProp, BorderRadiusProp, ColorProp, CssProperty, DisplayBox,
    DisplayOutside, DisplayProp, FontFamilyProp, FontSizeProp, FontWeightProp, HeightProp,
    MarginBlockProp, MarginProp, PaddingProp, TextDecorationProp, WidthProp,
};
use crate::utils::PrintableTree;

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

    pub fn to_box_tree(&self, draw_ctx: &pango::Context) -> Result<BoxTree> {
        BoxTree::build(self, draw_ctx)
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
            let children_num = node.borrow().children.len();
            for (i, child) in node.borrow().children.iter().enumerate() {
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

impl PrintableTree for RenderTree {}

#[derive(Debug)]
pub struct RenderNode {
    pub dom_node: Rc<RefCell<DomNode>>,
    pub style: ComputedStyle,
    pub children: Vec<Rc<RefCell<Self>>>,
}

impl RenderNode {
    pub fn build(
        node: Rc<RefCell<DomNode>>,
        style_sheets: &Vec<StyleSheet>,
        parent_style: Option<ComputedStyle>,
    ) -> Result<Option<Self>> {
        // Omit nodes that are not rendered.
        match &node.borrow().node_type {
            NodeType::DocumentType(_) | NodeType::Comment(_) => {
                return Ok(None);
            }
            _ => {}
        }

        let computed_style = match &node.borrow().node_type {
            NodeType::Element(_) => {
                // https://www.w3.org/TR/css-cascade-3/#value-stages
                apply_filtering(Rc::clone(&node), style_sheets)
                    .apply_cascading()
                    .apply_defaulting(&parent_style)?
                    .apply_computing()
            }
            NodeType::Text(_) => {
                if parent_style.is_some() {
                    let mut style = parent_style.as_ref().unwrap().clone();
                    style.display.outside = DisplayOutside::Inline;
                    style
                } else {
                    unreachable!()
                }
            }
            _ => ComputedStyle::default(),
        };

        // All elements with a value of none for the display property and their descendants are not rendered.
        // Some elements such as <meta>, <title>, <script>, <style> are marked as `none` in the UA style sheet.
        // https://developer.mozilla.org/en-US/docs/Web/CSS/display#none
        if computed_style.display.display_box == Some(DisplayBox::None) {
            return Ok(None);
        }

        let child_nodes = node
            .borrow()
            .children
            .iter()
            .map(|child| Self::build(Rc::clone(child), style_sheets, Some(computed_style.clone())))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            // Skip the children that are not rendered.
            .filter(|child| child.is_some())
            .map(|child| Rc::new(RefCell::new(child.unwrap())))
            .collect::<Vec<_>>();

        Ok(Some(Self {
            dom_node: Rc::clone(&node),
            style: computed_style,
            children: child_nodes,
        }))
    }

    pub fn get_display_type(&self) -> DisplayOutside {
        self.style.display.outside
    }

    pub fn get_tag_name(&self) -> Option<String> {
        self.dom_node.borrow().get_tag_name()
    }
}

impl fmt::Display for RenderNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.dom_node.borrow().node_type {
            NodeType::Element(elm) => write!(f, "{}, Computed( {})", elm, self.style),
            NodeType::Text(_) => write!(
                f,
                "{:?}, Computed( {})",
                self.dom_node.borrow().node_type,
                self.style
            ),
            _ => write!(f, "{:?}", self.dom_node.borrow().node_type),
        }
    }
}

/// Returns all declared values that match the node.
/// https://www.w3.org/TR/css-cascade-3/#filtering
fn apply_filtering(node: Rc<RefCell<DomNode>>, style_sheets: &[StyleSheet]) -> DeclaredStyle {
    let mut declared_values = DeclaredStyle::new();

    // As for the order of appearance in the subsequent cascading stage, the declarations from style sheets independently
    // linked by the originating document are treated as if they were concatenated in linking order, as determined by the host document language.
    style_sheets.iter().for_each(|style_sheet| {
        style_sheet.rules.iter().for_each(|rule| {
            let selectors = rule.get_matched_selectors(Rc::clone(&node));
            if selectors.is_some() {
                let Rule::QualifiedRule(qualified_rule) = rule else {
                    unreachable!();
                };
                for selector in selectors.unwrap() {
                    declared_values.add(selector, &qualified_rule.declarations);
                }
            }
        });
    });

    declared_values
}

/// https://www.w3.org/TR/css-cascade-3/#declared
#[derive(Debug)]
pub struct DeclaredStyle {
    pub values: Vec<(Selector, Vec<Declaration>)>,
}

impl DeclaredStyle {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn add(&mut self, selector: Selector, declarations: &[Declaration]) {
        self.values.push((selector, declarations.to_vec()));
    }

    /// Returns the cascaded values, which are the declared values that "win" the cascade.
    /// There is at most one cascaded value per property per element.
    /// https://www.w3.org/TR/css-cascade-3/#cascading
    pub fn apply_cascading(&self) -> CascadedStyle {
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
        let mut cascaded_values = CascadedStyle::new();
        for declarations in sorted_list.iter().map(|(_, declarations, _)| declarations) {
            for declaration in declarations {
                // The higher-priority declarations are placed first in the table,
                // and declarations placed later in the table that have lower-priority
                // with the same name are ignored.
                cascaded_values.add(&declaration.name, &declaration.value);
            }
        }

        cascaded_values
    }
}

/// https://www.w3.org/TR/css-cascade-3/#cascaded
#[derive(Debug)]
pub struct CascadedStyle {
    pub values: IndexMap<String, Vec<ComponentValue>>,
}

impl CascadedStyle {
    pub fn new() -> Self {
        Self {
            values: IndexMap::new(),
        }
    }

    /// Keeps the order of addition.
    pub fn add(&mut self, name: &str, values: &[ComponentValue]) {
        self.values
            .entry(name.to_string())
            .or_insert_with(|| values.to_vec());
    }

    /// Returns the specified values. All properties are set to their initial values or inherited values.
    /// https://www.w3.org/TR/css-cascade-3/#defaulting
    pub fn apply_defaulting(&self, parent_style: &Option<ComputedStyle>) -> Result<SpecifiedStyle> {
        let mut specified_values = SpecifiedStyle::new();

        specified_values.initialize();

        if parent_style.is_some() {
            specified_values.inherit(parent_style.as_ref().unwrap());
        }

        specified_values.set_from(self);

        Ok(specified_values)
    }
}

/// https://www.w3.org/TR/css-cascade-3/#specified
#[derive(Clone, Debug, Default)]
pub struct SpecifiedStyle {
    pub background_color: Option<BackGroundColorProp>,
    pub color: Option<ColorProp>,
    pub display: Option<DisplayProp>,
    pub font_family: Option<FontFamilyProp>,
    pub font_size: Option<FontSizeProp>,
    pub font_weight: Option<FontWeightProp>,
    pub text_decoration: Option<TextDecorationProp>,
    pub margin: Option<MarginProp>,
    pub margin_block: Option<MarginBlockProp>,
    pub border: Option<BorderProp>,
    pub padding: Option<PaddingProp>,
    pub width: Option<WidthProp>,
    pub height: Option<HeightProp>,
    pub border_radius: Option<BorderRadiusProp>,
}

impl SpecifiedStyle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the initial values for the properties.
    pub fn initialize(&mut self) {
        // todo: Add more properties.
        self.background_color = Some(BackGroundColorProp::default());
        self.color = Some(ColorProp::default());
        self.display = Some(DisplayProp::default());
        self.font_family = Some(FontFamilyProp::default());
        self.font_size = Some(FontSizeProp::default());
        self.font_weight = Some(FontWeightProp::default());
        self.text_decoration = Some(TextDecorationProp::default());
        self.margin = Some(MarginProp::default());
        self.margin_block = Some(MarginBlockProp::default());
        self.border = Some(BorderProp::default());
        self.padding = Some(PaddingProp::default());
        self.width = Some(WidthProp::default());
        self.height = Some(HeightProp::default());
        self.border_radius = Some(BorderRadiusProp::default());
    }

    /// Sets the inherited values for all "inherited properties".
    /// The values inherited from the parent element must be the computed values.
    pub fn inherit(&mut self, parent_values: &ComputedStyle) {
        self.color = Some(parent_values.color.clone());
        self.font_family = Some(parent_values.font_family.clone());
        self.font_size = Some(parent_values.font_size.clone());
        self.font_weight = Some(parent_values.font_weight.clone());
    }

    // Assumes that the computed values have been initialized and inherited.
    pub fn set_from(&mut self, cascaded_values: &CascadedStyle) {
        let mut cascaded_values = cascaded_values.values.clone();
        // The higher priority styles are placed first in the values, so
        // it must be reversed to process according to the priority.
        // O(n) time
        cascaded_values.reverse();
        for (name, values) in &cascaded_values {
            match name.as_str() {
                "background-color" => {
                    if let Ok(v) = BackGroundColorProp::parse(values) {
                        self.background_color = Some(v);
                    }
                }
                "color" => {
                    if let Ok(v) = ColorProp::parse(values) {
                        self.color = Some(v);
                    }
                }
                "display" => {
                    if let Ok(v) = DisplayProp::parse(values) {
                        self.display = Some(v);
                    }
                }
                "font-family" => {
                    if let Ok(v) = FontFamilyProp::parse(values) {
                        self.font_family = Some(v);
                    }
                }
                "font-size" => {
                    if let Ok(v) = FontSizeProp::parse(values) {
                        self.font_size = Some(v);
                    }
                }
                "font-weight" => {
                    if let Ok(v) = FontWeightProp::parse(values) {
                        self.font_weight = Some(v);
                    }
                }
                "text-decoration" => {
                    if let Ok(v) = TextDecorationProp::parse(values) {
                        self.text_decoration = Some(v);
                    }
                }
                "margin" => {
                    if let Ok(v) = MarginProp::parse(values) {
                        self.margin = Some(v);
                    }
                    // Assume that the margin-block-start and margin-block-end values
                    // are the same as the margin-top and margin-bottom values.
                    // todo: Handle the direction of the text.
                    self.margin_block.as_mut().unwrap().start =
                        self.margin.as_ref().unwrap().top.clone();
                    self.margin_block.as_mut().unwrap().end =
                        self.margin.as_ref().unwrap().bottom.clone();
                }
                "margin-block" => {
                    if let Ok(v) = MarginBlockProp::parse(values) {
                        self.margin_block = Some(v);
                    }
                    // Assume that the margin-block-start and margin-block-end values
                    // are the same as the margin-top and margin-bottom values.
                    // todo: Handle the direction of the text.
                    self.margin.as_mut().unwrap().top =
                        self.margin_block.as_ref().unwrap().start.clone();
                    self.margin.as_mut().unwrap().bottom =
                        self.margin_block.as_ref().unwrap().end.clone();
                }
                "border" => {
                    if let Ok(v) = BorderProp::parse(values) {
                        self.border = Some(v);
                    }
                }
                "padding" => {
                    if let Ok(v) = PaddingProp::parse(values) {
                        self.padding = Some(v);
                    }
                }
                "width" => {
                    if let Ok(v) = WidthProp::parse(values) {
                        self.width = Some(v);
                    }
                }
                "height" => {
                    if let Ok(v) = HeightProp::parse(values) {
                        self.height = Some(v);
                    }
                }
                "border-radius" => {
                    if let Ok(v) = BorderRadiusProp::parse(values) {
                        self.border_radius = Some(v);
                    }
                }
                _ => {}
            }
        }
    }

    /// Converts the relative values to absolute values.
    /// https://www.w3.org/TR/css-cascade-3/#computed
    pub fn apply_computing(&self) -> ComputedStyle {
        let mut v = self.clone();

        Self::compute_earlier(&mut v, self);
        let earlier_style = v.clone();
        Self::compute_later(&mut v, &earlier_style);

        ComputedStyle {
            background_color: v.background_color.unwrap(),
            color: v.color.unwrap(),
            display: v.display.unwrap(),
            font_family: v.font_family.unwrap(),
            font_size: v.font_size.unwrap(),
            font_weight: v.font_weight.unwrap(),
            text_decoration: v.text_decoration.unwrap(),
            margin: v.margin.unwrap(),
            margin_block: v.margin_block.unwrap(),
            border: v.border.unwrap(),
            padding: v.padding.unwrap(),
            width: v.width.unwrap(),
            height: v.height.unwrap(),
            border_radius: v.border_radius.unwrap(),
        }
    }

    /// Computes the properties whose values are used to compute other properties.
    fn compute_earlier(v: &mut Self, initialized_style: &Self) {
        Self::compute_property(&mut v.color, Some(initialized_style));
        Self::compute_property(&mut v.font_size, Some(initialized_style));
        Self::compute_property(&mut v.display, None);
    }

    /// Computes the properties that require some computed values.
    fn compute_later(v: &mut Self, earlier_style: &Self) {
        Self::compute_property(&mut v.background_color, Some(earlier_style));
        Self::compute_property(&mut v.font_family, Some(earlier_style));
        Self::compute_property(&mut v.font_weight, Some(earlier_style));
        Self::compute_property(&mut v.text_decoration, Some(earlier_style));
        Self::compute_property(&mut v.margin, Some(earlier_style));
        Self::compute_property(&mut v.margin_block, Some(earlier_style));
        Self::compute_property(&mut v.border, Some(earlier_style));
        Self::compute_property(&mut v.padding, Some(earlier_style));
        Self::compute_property(&mut v.width, Some(earlier_style));
        Self::compute_property(&mut v.height, Some(earlier_style));
        Self::compute_property(&mut v.border_radius, Some(earlier_style));
    }

    fn compute_property(prop: &mut Option<impl CssProperty>, current_style: Option<&Self>) {
        if let Err(e) = prop
            .as_mut()
            .context(anyhow!("Uninitialized property detected while computing."))
            .unwrap()
            .compute(current_style)
        {
            eprintln!("{e}");
        }
    }
}

/// https://www.w3.org/TR/css-cascade-3/#computed
#[derive(Clone, Debug, Default)]
pub struct ComputedStyle {
    pub background_color: BackGroundColorProp,
    pub color: ColorProp,
    pub display: DisplayProp,
    pub font_family: FontFamilyProp,
    pub font_size: FontSizeProp,
    pub font_weight: FontWeightProp,
    pub text_decoration: TextDecorationProp,
    pub margin: MarginProp,
    pub margin_block: MarginBlockProp,
    pub border: BorderProp,
    pub padding: PaddingProp,
    pub width: WidthProp,
    pub height: HeightProp,
    pub border_radius: BorderRadiusProp,
}

impl fmt::Display for ComputedStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut style_str = String::new();
        style_str.push_str(&format!("background-color: {}; ", self.background_color));
        style_str.push_str(&format!("color: {}; ", self.color));
        style_str.push_str(&format!("display: {}; ", self.display));
        style_str.push_str(&format!("font-family: {}; ", self.font_family));
        style_str.push_str(&format!("font-size: {}; ", self.font_size));
        style_str.push_str(&format!("font-weight: {}; ", self.font_weight));
        style_str.push_str(&format!("text-decoration: {}; ", self.text_decoration));
        style_str.push_str(&format!("margin: {}; ", self.margin));
        style_str.push_str(&format!("margin-block: {}; ", self.margin_block));
        style_str.push_str(&format!("border: {}; ", self.border));
        style_str.push_str(&format!("padding: {}; ", self.padding));
        style_str.push_str(&format!("width: {}; ", self.width));
        style_str.push_str(&format!("height: {}; ", self.height));
        style_str.push_str(&format!("border-radius: {}", self.border_radius));
        write!(f, "{}", style_str)
    }
}
