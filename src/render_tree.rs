use std::fmt;
use std::rc::Rc;
use std::{cell::RefCell, collections::HashMap};

use anyhow::{bail, Context, Ok, Result};

use crate::css::cssom::{ComponentValue, Declaration, Rule, StyleSheet};
use crate::css::selector::Selector;
use crate::css::tokenizer::{CssToken, NumericType};
use crate::html::dom::{DocumentTree, DomNode, Element, NodeType};

#[derive(Debug)]
pub struct RenderNode {
    node: Rc<RefCell<DomNode>>,
    style: ComputedValues,
    child_nodes: Vec<Rc<RefCell<RenderNode>>>,
}

impl RenderNode {
    pub fn build(
        node: Rc<RefCell<DomNode>>,
        style_sheets: &Vec<StyleSheet>,
        parent_style: Option<SpecifiedValues>,
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

        let child_nodes = node
            .borrow()
            .child_nodes
            .iter()
            .map(|child| Self::build(Rc::clone(child), style_sheets, Some(style.clone())))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            // Skip the children that are not rendered
            .filter(|child| child.is_some())
            .map(|child| Rc::new(RefCell::new(child.unwrap())))
            .collect::<Vec<_>>();

        Ok(Some(Self {
            node: Rc::clone(&node),
            style: style.compute()?,
            child_nodes,
        }))
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
    fn compute(&self) -> Result<ComputedValues> {
        let mut computed_values = ComputedValues::new(HashMap::new());

        for (name, value) in &self.values {
            match name.as_str() {
                // https://developer.mozilla.org/en-US/docs/Web/CSS/background-color
                // https://developer.mozilla.org/en-US/docs/Web/CSS/color
                "background-color" | "color" => {
                    if value.len() != 1 {
                        bail!("The {} property must have exactly one value.", name);
                    }
                    let value = value.first().unwrap();
                    computed_values.values.insert(
                        name.to_string(),
                        match value {
                            ComponentValue::PreservedToken(token) => match token {
                                CssToken::Ident(color) => match color.as_str() {
                                    "black" => ComputedValue::Color(0, 0, 0),
                                    "gray" => ComputedValue::Color(128, 128, 128),
                                    "white" => ComputedValue::Color(255, 255, 255),
                                    "red" => ComputedValue::Color(255, 0, 0),
                                    "purple" => ComputedValue::Color(128, 0, 128),
                                    "green" => ComputedValue::Color(0, 128, 0),
                                    "yellowgreen" => ComputedValue::Color(154, 205, 50),
                                    "yellow" => ComputedValue::Color(255, 255, 0),
                                    "blue" => ComputedValue::Color(0, 0, 255),
                                    "aqua" => ComputedValue::Color(0, 255, 255),
                                    "orange" => ComputedValue::Color(255, 165, 0),
                                    "brown" => ComputedValue::Color(165, 42, 42),
                                    _ => unimplemented!(),
                                },
                                _ => unimplemented!(),
                            },
                            ComponentValue::Function { name, values } => {
                                let args = values
                                    .iter()
                                    .filter(|value| {
                                        **value
                                            != ComponentValue::PreservedToken(CssToken::Whitespace)
                                            && **value
                                                != ComponentValue::PreservedToken(CssToken::Comma)
                                    })
                                    .collect::<Vec<_>>();
                                match name.as_str() {
                                    "rgb" => {
                                        let r = match args.first().unwrap() {
                                            ComponentValue::PreservedToken(token) => match token {
                                                CssToken::Number(number) => match number {
                                                    NumericType::Integer(integer) => *integer as u8,
                                                    NumericType::Number(float) => *float as u8,
                                                },
                                                _ => unimplemented!(),
                                            },
                                            _ => unimplemented!(),
                                        };
                                        let g = match args.get(1).unwrap() {
                                            ComponentValue::PreservedToken(token) => match token {
                                                CssToken::Number(number) => match number {
                                                    NumericType::Integer(integer) => *integer as u8,
                                                    NumericType::Number(float) => *float as u8,
                                                },
                                                _ => unimplemented!(),
                                            },
                                            _ => unimplemented!(),
                                        };
                                        let b = match args.get(2).unwrap() {
                                            ComponentValue::PreservedToken(token) => match token {
                                                CssToken::Number(number) => match number {
                                                    NumericType::Integer(integer) => *integer as u8,
                                                    NumericType::Number(float) => *float as u8,
                                                },
                                                _ => unimplemented!(),
                                            },
                                            _ => unimplemented!(),
                                        };
                                        ComputedValue::Color(r, g, b)
                                    }
                                    _ => unimplemented!(),
                                }
                            }
                            _ => unimplemented!(),
                        },
                    );
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/font-size
                "font-size" => {
                    if value.len() != 1 {
                        bail!("The font-size property must have exactly one value.");
                    }
                    let value = value.first().unwrap();
                    match value {
                        ComponentValue::PreservedToken(token) => match &token {
                            CssToken::Ident(size) => match size.as_str() {
                                "xx-small" | "x-small" | "small" | "medium" | "large"
                                | "x-large" | "xx-large" => {
                                    computed_values.values.insert(
                                        name.to_string(),
                                        ComputedValue::String(size.to_string()),
                                    );
                                }
                                _ => {
                                    bail!("Unexpected value for the font-size property: {:?}", size)
                                }
                            },
                            CssToken::Number(number) => match number {
                                NumericType::Integer(integer) => {
                                    computed_values.values.insert(
                                        name.to_string(),
                                        ComputedValue::Length(*integer as f32, "px".to_string()),
                                    );
                                }
                                NumericType::Number(float) => {
                                    computed_values.values.insert(
                                        name.to_string(),
                                        ComputedValue::Length(*float, "px".to_string()),
                                    );
                                }
                            },
                            _ => unimplemented!(),
                        },
                        _ => unimplemented!(),
                    }
                }

                // https://drafts.csswg.org/css-display/#the-display-properties
                "display" => {
                    if value.len() != 1 {
                        unimplemented!();
                    }
                    let value = value.first().unwrap();
                    computed_values.values.insert(
                        name.to_string(),
                        match value {
                            ComponentValue::PreservedToken(token) => match &token {
                                CssToken::Ident(keyword) => ComputedValue::String(keyword.clone()),
                                _ => unimplemented!(),
                            },
                            _ => bail!("Unexpected value for the display property: {:?}", value),
                        },
                    );
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/width
                // https://developer.mozilla.org/en-US/docs/Web/CSS/height
                "width" | "height" => {
                    if value.len() != 1 {
                        bail!("The {} property must have exactly one value.", name);
                    }
                    let value = value.first().unwrap();
                    computed_values.values.insert(
                        name.to_string(),
                        match value {
                            ComponentValue::PreservedToken(token) => match &token {
                                CssToken::Ident(keyword) => match keyword.as_str() {
                                    "auto" => ComputedValue::String(keyword.clone()),
                                    _ => unimplemented!(),
                                },
                                CssToken::Number(number) => match number {
                                    NumericType::Integer(integer) => {
                                        ComputedValue::Length(*integer as f32, "px".to_string())
                                    }
                                    NumericType::Number(float) => {
                                        ComputedValue::Length(*float, "px".to_string())
                                    }
                                },
                                CssToken::Percentage(percentage) => {
                                    ComputedValue::Percentage(*percentage)
                                }
                                CssToken::Dimension(value, unit) => match unit.as_str() {
                                    "px" => match value {
                                        NumericType::Integer(integer) => {
                                            ComputedValue::Length(*integer as f32, "px".to_string())
                                        }
                                        NumericType::Number(float) => {
                                            ComputedValue::Length(*float, "px".to_string())
                                        }
                                    },
                                    _ => unimplemented!(),
                                },
                                _ => unimplemented!(),
                            },
                            _ => unimplemented!(),
                        },
                    );
                }

                // https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration
                "text-decoration" => {
                    if value.len() != 1 {
                        unimplemented!();
                    }
                    let value = value.first().unwrap();
                    computed_values.values.insert(
                        name.to_string(),
                        match value {
                            ComponentValue::PreservedToken(token) => match &token {
                                CssToken::Ident(keyword) => ComputedValue::String(keyword.clone()),
                                _ => unimplemented!(),
                            },
                            _ => bail!(
                                "Unexpected value for the text-decoration property: {:?}",
                                value
                            ),
                        },
                    );
                }

                _ => unimplemented!(),
            }
        }

        Ok(computed_values)
    }
}

/// https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Types
#[derive(Debug)]
pub enum ComputedValue {
    String(String),
    Percentage(f32),
    Length(f32, String),
    Color(u8, u8, u8),
}

impl fmt::Display for ComputedValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ComputedValue::String(keyword) => write!(f, "{}", keyword),
            ComputedValue::Percentage(percentage) => write!(f, "{}%", percentage),
            ComputedValue::Length(length, unit) => write!(f, "{}{}", length, unit),
            ComputedValue::Color(r, g, b) => {
                if let Some(color_name) = get_color_name(ComputedValue::Color(*r, *g, *b)) {
                    write!(f, "#{:02x}{:02x}{:02x} ({})", r, g, b, color_name)
                } else {
                    write!(f, "#{:02x}{:02x}{:02x}", r, g, b)
                }
            }
        }
    }
}

fn get_color_name(val: ComputedValue) -> Option<String> {
    match val {
        ComputedValue::Color(r, g, b) => Some(match (r, g, b) {
            (0, 0, 0) => "black".to_string(),
            (128, 128, 128) => "gray".to_string(),
            (255, 255, 255) => "white".to_string(),
            (255, 0, 0) => "red".to_string(),
            (128, 0, 128) => "purple".to_string(),
            (0, 128, 0) => "green".to_string(),
            (154, 205, 50) => "yellowgreen".to_string(),
            (255, 255, 0) => "yellow".to_string(),
            (0, 0, 255) => "blue".to_string(),
            (0, 255, 255) => "aqua".to_string(),
            (255, 165, 0) => "orange".to_string(),
            (165, 42, 42) => "brown".to_string(),
            _ => return None,
        }),
        _ => None,
    }
}

#[derive(Debug)]
pub struct ComputedValues {
    values: HashMap<String, ComputedValue>,
}

impl ComputedValues {
    pub fn new(values: HashMap<String, ComputedValue>) -> Self {
        Self { values }
    }
}

impl fmt::Display for ComputedValues {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut style_str = String::new();
        for (name, value) in &self.values {
            style_str.push_str(&format!("{}: {}; ", name, value));
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
    root: Rc<RefCell<RenderNode>>,
}

impl RenderTree {
    pub fn build(document_tree: DocumentTree, style_sheets: Vec<StyleSheet>) -> Result<Self> {
        Ok(Self {
            root: Rc::new(RefCell::new(
                RenderNode::build(Rc::clone(&document_tree.root), &style_sheets, None)?
                    .context("Failed to build the render tree.")?,
            )),
        })
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
