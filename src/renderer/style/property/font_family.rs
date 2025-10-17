use std::fmt;

use crate::{
    error::{Error, Result},
    renderer::{
        css::{cssom::ComponentValue, token::CssToken},
        style::{
            SpecifiedStyle,
            property::{CssProperty, CssValue},
        },
    },
};

/// https://developer.mozilla.org/en-US/docs/Web/CSS/font-family
#[derive(Clone, Debug, PartialEq)]
pub struct FontFamilyProp {
    pub family: Vec<CssValue>,
}

impl fmt::Display for FontFamilyProp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.family
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

impl Default for FontFamilyProp {
    fn default() -> Self {
        Self {
            // Define the default font family of the browser.
            family: vec![
                CssValue::String("Times New Roman".to_string()),
                CssValue::Ident("serif".to_string()),
            ],
        }
    }
}

impl CssProperty for FontFamilyProp {
    // font-family =
    //    [ <family-name> | <generic-family> ]#
    // todo
    fn parse(values: &[ComponentValue]) -> Result<Self> {
        let mut values = values.iter().cloned().peekable();
        let mut family = Vec::new();
        while values.peek().is_some() {
            while values
                .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                .is_some()
            {}
            values.next_if_eq(&ComponentValue::PreservedToken(CssToken::Comma));
            while values
                .next_if_eq(&ComponentValue::PreservedToken(CssToken::Whitespace))
                .is_some()
            {}
            if let Some(ComponentValue::PreservedToken(v)) = values.next() {
                match v {
                    CssToken::Ident(font) | CssToken::String(font) => {
                        family.push(CssValue::String(font.to_string()));
                    }
                    _ => {
                        return Err(Error::CssProperty(format!(
                            "Expected <family-name> or <generic-family> but found: {:?}",
                            v
                        )));
                    }
                }
            }
        }
        Ok(Self { family })
    }

    fn compute(&mut self, _: Option<&SpecifiedStyle>, _: i32, _: i32) -> Result<&Self> {
        Ok(self)
    }
}

impl FontFamilyProp {
    pub fn to_name_list(&self) -> Result<Vec<String>> {
        self.family
            .iter()
            .map(|v| v.to_name())
            .collect::<Result<Vec<String>>>()
    }
}
