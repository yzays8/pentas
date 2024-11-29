mod css;
mod html;
mod layout;
mod style;
mod utils;

use anyhow::{Context, Result};

use crate::app::DEFAULT_WINDOW_WIDTH;
use css::get_ua_style_sheet;
use css::parser::parse_css;
use css::token::tokenize_css;
use html::dom::DocumentTree;
use html::parser::HtmlParser;
use html::token::HtmlTokenizer;

#[derive(Debug, Clone)]
pub enum RenderObject {
    Text {
        text: String,
        x: f64,
        y: f64,
        size: f64,
        color: (f64, f64, f64),
    },
    Rectangle {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: (f64, f64, f64),
    },
}

#[derive(Debug)]
pub struct Renderer {
    html_path: Option<String>,
    css_path: Option<String>,
    is_tracing_enabled: bool,
}

impl Renderer {
    pub fn new(
        html_path: Option<String>,
        css_path: Option<String>,
        is_tracing_enabled: bool,
    ) -> Self {
        Self {
            html_path,
            css_path,
            is_tracing_enabled,
        }
    }

    #[allow(dead_code)]
    pub fn set_html_path(&mut self, html_path: String) {
        self.html_path = Some(html_path);
    }

    #[allow(dead_code)]
    pub fn set_css_path(&mut self, css_path: String) {
        self.css_path = Some(css_path);
    }

    #[allow(dead_code)]
    pub fn set_tracing_enabled(&mut self, is_tracing_enabled: bool) {
        self.is_tracing_enabled = is_tracing_enabled;
    }

    // Returns the render objects.
    pub fn run(&self) -> Result<Vec<RenderObject>> {
        let html_path = self
            .html_path
            .as_ref()
            .context("HTML file path is not provided.")?;
        let (doc_root, style_sheets) =
            HtmlParser::new(HtmlTokenizer::new(&std::fs::read_to_string(html_path)?)).parse()?;
        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(style_sheets)
            .collect::<Vec<_>>();

        if self.is_tracing_enabled {
            Ok(DocumentTree::build(doc_root)?
                .print_in_chain()
                .to_render_tree(style_sheets)?
                .print_in_chain()
                .to_box_tree()?
                .print_in_chain()
                .clean_up()?
                .print_in_chain()
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .print_in_chain()
                .to_render_objects())
        } else {
            Ok(DocumentTree::build(doc_root)?
                .to_render_tree(style_sheets)?
                .to_box_tree()?
                .clean_up()?
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .to_render_objects())
        }
    }

    /// Displays the HTML as a box tree.
    pub fn display_html(&self) -> Result<()> {
        let html_path = self
            .html_path
            .as_ref()
            .context("HTML file path is not provided.")?;
        let (doc_root, style_sheets) =
            HtmlParser::new(HtmlTokenizer::new(&std::fs::read_to_string(html_path)?)).parse()?;

        let style_sheets = std::iter::once(get_ua_style_sheet()?)
            .chain(style_sheets)
            .collect::<Vec<_>>();

        if self.is_tracing_enabled {
            DocumentTree::build(doc_root)?
                .print_in_chain()
                .to_render_tree(style_sheets)?
                .print_in_chain()
                .to_box_tree()?
                .print_in_chain()
                .clean_up()?
                .print_in_chain()
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .print();
        } else {
            DocumentTree::build(doc_root)?
                .to_render_tree(style_sheets)?
                .to_box_tree()?
                .clean_up()?
                .layout(DEFAULT_WINDOW_WIDTH as f32)?
                .print();
        }

        Ok(())
    }

    /// Displays the CSS as a style sheet.
    pub fn display_css(&self) -> Result<()> {
        let css_path = self
            .css_path
            .as_ref()
            .context("CSS file path is not provided.")?;
        parse_css(&tokenize_css(&std::fs::read_to_string(css_path)?)?)?.print();
        Ok(())
    }
}
