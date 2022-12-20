use cooklang_rs::parser::{parse, Part};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Quantity {
    Int(i32),
    Str(String),
    Float(f32),
}

impl Quantity {
    fn to_string(self: &Self) -> String {
        match self {
            Quantity::Int(a) => {a.to_string()},
            Quantity::Str(a) => a.clone(),
            Quantity::Float(a) => a.to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct StepTNQU {
    t: String,
    name: String,
    quantity: Quantity,
    units: String,
}

#[derive(Deserialize, Debug)]
struct StepTNQ {
    t: String,
    name: String,
    quantity: Quantity,
}

#[derive(Deserialize, Debug)]
struct StepTV {
    t: String,
    value: String,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Step {
    StepTV(StepTV),
    StepTNQU(StepTNQU),
    StepTNQ(StepTNQ),
}

#[derive(Deserialize, Debug)]
struct Result {
    steps: Vec<Vec<Step>>,
    metadata: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct Test {
    source: String,
    result: Result,
}

#[derive(Deserialize, Debug)]
struct CanonicalTests {
    version: u8,
    tests: HashMap<String, Test>,
}

#[test]
fn test_canonical() {
    let tests: CanonicalTests = serde_yaml::from_str(include_str!("canonical.yaml")).unwrap();
    println!("canonical tests version {}", tests.version);
    for (name, mut test) in tests.tests {
        println!("test {name}");
        if name == "testTimerFractional" || name == "testFractionsWithSpaces" || name == "testFractions"{
            // skip
            continue;
        }
        let out = parse(test.source);

        let mut step_indice = 0;
        for out_step in out.into_iter() {
            // 2 cases : 
            // - a vect with a single metadata
            // - a vect with a multiple other steps
            if out_step.len() == 1{
                if let Part::Metadata(metadata) = out_step.get(0).unwrap() {
                    println!("metadata key {}", metadata.key);
                    println!("metadata possible keys {:?}", test.result.metadata.keys());
                    assert!(test.result.metadata.contains_key(&metadata.key)); 
                    let v = test.result.metadata.remove(&metadata.key).unwrap();
                    assert_eq!(metadata.value, v);
                    continue;
                }
            }
            // compare with test.result.steps.get(step_indice)
            let steps = test.result.steps.get(step_indice).unwrap();
            step_indice += 1;
            println!("{:?}, \n{:?}", steps, out_step);
            println!("{:?}, {:?}", steps.len(), out_step.len());
            assert_eq!(steps.len(), out_step.len());
            for (a, b) in steps.into_iter().zip(out_step.into_iter()) {
                match (a, b) {
                    (Step::StepTV(t), Part::Text(string)) => {
                        assert_eq!(t.t, "text");
                        assert_eq!(t.value.trim(), string);
                    },
                    (Step::StepTNQU(t), Part::Timer(t2)) => {
                        assert_eq!(t2.name, t.name);
                        assert_eq!(t2.units, t.units);
                        assert_eq!(t.t, "timer");
                        assert_eq!(t2.quantity, t.quantity.to_string());
                    },
                    (Step::StepTNQ(t), Part::Cookware(cookware)) => {
                        assert_eq!(t.t, "cookware");
                        assert_eq!(cookware.name, t.name);
                        if cookware.quantity != "" {
                            assert_eq!(cookware.quantity, t.quantity.to_string());
                        } else {
                            assert_eq!(t.quantity.to_string(), "1");
                        }
                    },
                    (Step::StepTNQU(t), Part::Ingredient(ingredient)) => {
                        assert_eq!(t.t, "ingredient");
                        assert_eq!(ingredient.name, t.name);
                        assert_eq!(ingredient.units, t.units);
                        if ingredient.quantity != "" {
                            assert_eq!(ingredient.quantity, t.quantity.to_string());
                        } else {
                            assert_eq!(t.quantity.to_string(), "some");
                        }
                    },
                    _ => panic!()
                }
            }
        }

        // check that all metadata has been processed
        assert_eq!(test.result.metadata.len(), 0)
        
    }
}
