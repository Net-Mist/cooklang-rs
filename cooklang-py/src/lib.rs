use std::collections::HashMap;

use cooklang_rs::parser;
use cooklang_rs::parser::Part;
use pyo3::prelude::*;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn parse(text: String) -> Vec<Vec<HashMap<String, String>>> {
    let r = parser::parse(&text).unwrap_or_default();
    let mut out = Vec::new();
    for l in r.into_iter() {
        let mut out_line = Vec::new();
        for p in l.into_iter() {
            let mut i = HashMap::new();
            match p {
                Part::Metadata(metadata) => {
                    i.insert("type".to_string(), "metadata".to_string());
                    i.insert("key".to_string(), metadata.key);
                    i.insert("value".to_string(), metadata.value);
                    out_line.push(i)
                }
                Part::Cookware(cookware) => {
                    i.insert("type".to_string(), "cookware".to_string());
                    i.insert("name".to_string(), cookware.name);
                    i.insert("quantity".to_string(), cookware.quantity);
                    out_line.push(i)
                }
                Part::Timer(timer) => {
                    i.insert("type".to_string(), "timer".to_string());
                    i.insert("name".to_string(), timer.name);
                    i.insert("quantity".to_string(), timer.quantity);
                    i.insert("units".to_string(), timer.units);
                    out_line.push(i)
                }
                Part::Ingredient(ingredient) => {
                    i.insert("type".to_string(), "ingredient".to_string());
                    i.insert("name".to_string(), ingredient.name);
                    i.insert("quantity".to_string(), ingredient.quantity);
                    i.insert("units".to_string(), ingredient.units);
                    out_line.push(i)
                }
                Part::Text(string) => {
                    i.insert("text".to_string(), string);
                    out_line.push(i)
                }
            }
        }
        out.push(out_line);
    }

    out
}

/// A Python module implemented in Rust.
#[pymodule]
fn cooklang(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}
