
use std::collections::HashMap;

use pyo3::prelude::*;
use cooklang_rs::parser::Part;
use cooklang_rs::parser;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn parse(text: &str) -> Vec<Vec<HashMap<&str, String>>> {
    let r = parser::parse(text.to_string());
    let mut out = Vec::new();
    for l in r.into_iter() {
        let mut out_line = Vec::new();
        for p in l.into_iter() {
            let mut i = HashMap::new();
            match p {
                Part::Metadata(metadata) => {
                    i.insert("type", "metadata".to_string());
                    i.insert("key", metadata.key);
                    i.insert("value", metadata.value);
                    out_line.push(i)
                },
                Part::Cookware(cookware) => {
                    i.insert("type", "cookware".to_string());
                    i.insert("name", cookware.name);
                    i.insert("quantity", cookware.quantity);
                    out_line.push(i)
                },
                Part::Timer(timer) => {
                    i.insert("type", "timer".to_string());
                    i.insert("name", timer.name);
                    i.insert("quantity", timer.quantity);
                    i.insert("units", timer.units);
                    out_line.push(i)
                },
                Part::Ingredient(ingredient) => {
                    i.insert("type", "ingredient".to_string());
                    i.insert("name", ingredient.name);
                    i.insert("quantity", ingredient.quantity);
                    i.insert("units", ingredient.units);
                    out_line.push(i)
                },
                Part::Text(string) => {
                    i.insert("text", string);
                    out_line.push(i)
                },
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