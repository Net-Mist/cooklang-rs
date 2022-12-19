use nom::bytes::complete::{tag, take, take_until, take_while, take_while1};

use nom::branch::alt;

use nom::character::complete::space0;
use nom::character::complete::space1;
use nom::combinator::eof;
use nom::combinator::map_res;
use nom::combinator::{map, value};
use nom::multi::{fold_many0, fold_many1, many_till};
use nom::sequence::{delimited, pair, preceded, separated_pair, tuple};
use nom::IResult;

#[derive(Debug, PartialEq, Eq)]
pub struct Metadata {
    pub key: String,
    pub value: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Ingredient {
    pub name: String,
    pub quantity: String,
    pub unit: String,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Cookware {
    pub name: String,
    pub quantity: String
}

#[derive(Debug, PartialEq, Eq)]
pub struct Timer {
    pub name: String,
    pub quantity: String,
    pub unit: String,
}

#[derive(Debug, PartialEq, Eq)]

pub enum Part {
    Metadata(Metadata),
    Cookware(Cookware),
    Timer(Timer),
    Ingredient(Ingredient),
    Text(String),
}

/// block comments = "[", "-", ? any character except "-" followed by "]" ?, "-", "]" ;
fn block_comment(input: &str) -> IResult<&str, &str> {
    value("", delimited(tag("[-"), take_until("-]"), tag("-]")))(input)
}

/// line comments       = "-", "-", text item, new line character ;
fn line_comment(input: &str) -> IResult<&str, &str> {
    value("", preceded(tag("--"), take_while(|c| c != '\n')))(input)
}

/// comments
fn comment(input: &str) -> IResult<&str, &str> {
    alt((block_comment, line_comment))(input)
}

pub fn remove_comment(input: String) -> String {
    let a = fold_many1(
        alt((comment, take(1u8))),
        String::new,
        |mut string, fragment| {
            string.push_str(fragment);
            string
        },
    )(&input);
    a.unwrap().1
}

/// spaces + one or many endline chars
fn end_line(input: &str) -> IResult<&str, &str> {
    preceded(space0, take_while1(|c| "\n\r".contains(c)))(input)
}

/// at least one space of tab
fn space(input: &str) -> IResult<&str, String> {
    map(space1, |c: &str| c.to_string())(input)
}

/// word      = { text item - white space - punctuation character }- ;
fn word(input: &str) -> IResult<&str, String> {
    map(take_while1(|c| !"~@#{ \t\n\r.,;".contains(c)), |c: &str| {
        c.to_string()
    })(input)
}

fn multiword(input: &str) -> IResult<&str, String> {
    map(
        tuple((
            word,
            fold_many0(tuple((space, word)), String::new, |mut string, (s1, s2)| {
                string.push_str(&s1);
                string.push_str(&s2);
                string
            }),
        )),
        |(s1, s2)| {
            let mut s = s1;
            s.push_str(&s2);
            s
        },
    )(input)
}

fn trim_spaces(s: &str) -> String {
    s.to_string().trim().to_string()
}

/// units    = { text item - "}" }- ;
/// spaces are trimmed
fn unit(input: &str) -> IResult<&str, String> {
    map(take_while(|c| !"\n\r}".contains(c)), trim_spaces)(input)
}

/// quantity = { text item - "%" - "}" }- ;
/// spaces are trimmed
fn quantity(input: &str) -> IResult<&str, String> {
    map(take_while(|c| !"\n\r}%".contains(c)), trim_spaces)(input)
}

/// amount   = {quantity | ( quantity, "%", units )} ;
fn amount(input: &str) -> IResult<&str, (String, String)> {
    delimited(
        tag("{"),
        alt((
            separated_pair(quantity, tag("%"), unit),
            map(quantity, |v| (v, "".to_string())),
        )),
        tag("}"),
    )(input)
}

fn multi_word_item(input: &str) -> IResult<&str, (String, String, String)> {
    map(pair(multiword, amount), |(word, (quantity, unit))| {
        (word, quantity, unit)
    })(input)
}

/// one word ingredient  = "@", ( word,                     [ "{", [ amount ], "}" ] ) ;
fn ingredient(input: &str) -> IResult<&str, Part> {
    preceded(
        preceded(space0, tag("@")),
        alt((
            map(multi_word_item, |(word, quantity, unit)| {
                Part::Ingredient(Ingredient {
                    name: word,
                    quantity,
                    unit,
                })
            }),
            map(word, |word| {
                Part::Ingredient(Ingredient {
                    name: word,
                    quantity: String::from(""),
                    unit: String::from(""),
                })
            }),
        )),
    )(input)
}

fn cookware(input: &str) -> IResult<&str, Part> {
    preceded(
        preceded(space0, tag("#")),
        alt((
            map(multi_word_item, |(word, quantity, _unit)| {
                Part::Cookware(Cookware {
                    name: word,
                    quantity: quantity
                })
            }),
            map(word, |word| {
                Part::Cookware(Cookware {
                    name: word,
                    quantity: "".to_string()
                })
            }),
        )),
    )(input)
}

fn timer(input: &str) -> IResult<&str, Part> {
    preceded(
        preceded(space0, tag("~")),
        alt((
            map(multi_word_item, |(word, quantity, unit)| {
                Part::Timer(Timer {
                    name: word,
                    quantity,
                    unit,
                })
            }),
            map(word, |word| {
                Part::Timer(Timer {
                    name: word,
                    quantity: String::from(""),
                    unit: String::from(""),
                })
            }),
            map(amount, |(quantity, unit)| {
                Part::Timer(Timer {
                    name: "".to_string(),
                    quantity,
                    unit,
                })
            })
        )),
    )(input)
}

fn metadata_tuple(input: &str) -> IResult<&str, (&str, &str, &str, &str)> {
    tuple((
        tag(">>"),
        take_while(|c| c != ':'),
        tag(":"),
        take_while(|c| c != '\n'),
    ))(input)
}

fn metadata(input: &str) -> IResult<&str, Part> {
    map(metadata_tuple, |(_, k, _, v)| {
        Part::Metadata(Metadata {
            key: trim_spaces(k),
            value: trim_spaces(v),
        })
    })(input)
}

fn text(input: &str) -> IResult<&str, Part> {
    map_res(take_while(|c| !"~@#{\n\r".contains(c)), |w: &str| {
        let s = trim_spaces(w);
        if s.is_empty() {
            return Err("no word");
        }
        Ok(Part::Text(s))
    })(input)
}

pub fn parse(input: String) -> Vec<Vec<Part>> {
    let pre_processed = remove_comment(input);
    let a = many_till(
        map(
            alt((
                map(metadata, |p| (vec![p], "a")),
                many_till(
                    alt((timer, cookware, ingredient, text)),
                    alt((end_line, eof)),
                ),
            )),
            |a| a.0,
        ),
        eof,
    )(pre_processed.trim());
    
    a.unwrap().1.0.into_iter().filter(|p| p.len()>0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_block_comment() {
        assert_eq!(block_comment("[- foo bar-]"), Ok(("", "")));
        assert_eq!(block_comment("[-foo-] bar"), Ok((" bar", "")));
    }

    #[test]
    fn test_line_comment() {
        assert_eq!(line_comment("--foo\n bar"), Ok(("\n bar", "")));
        assert_eq!(line_comment("-- foo bar"), Ok(("", "")));
    }

    #[test]
    fn test_comment() {
        assert_eq!(comment("--foo\n bar"), Ok(("\n bar", "")));
        assert_eq!(comment("-- foo bar"), Ok(("", "")));
        assert_eq!(comment("[- foo bar-]"), Ok(("", "")));
        assert_eq!(comment("[-foo-] bar"), Ok((" bar", "")));
        assert_eq!(comment("[-foo-] bar"), Ok((" bar", "")));
        assert_eq!(comment("[- -- foo-] bar"), Ok((" bar", "")));
        assert_eq!(comment("-- [- -- foo\n-] bar"), Ok(("\n-] bar", "")));
    }

    #[test]
    fn test_remove_comment() {
        assert_eq!(
            remove_comment(String::from("--foo\n bar")),
            String::from("\n bar")
        );
        assert_eq!(
            remove_comment(String::from("fo--foo\n bar")),
            String::from("fo\n bar")
        );
        assert_eq!(
            remove_comment(String::from("fo[-bar-]o")),
            String::from("foo")
        );
    }

    #[test]
    fn test_metadata() {
        assert_eq!(
            metadata(">> plop: coucou\nfoo"),
            Ok((
                "\nfoo",
                Part::Metadata(Metadata {
                    key: "plop".to_string(),
                    value: "coucou".to_string()
                })
            ))
        );
        assert_eq!(
            metadata(">> plop: coucou"),
            Ok((
                "",
                Part::Metadata(Metadata {
                    key: "plop".to_string(),
                    value: "coucou".to_string()
                })
            ))
        );
    }

    #[test]
    fn test_text() {
        assert_eq!(
            text("foo bar"),
            Ok(("", Part::Text(String::from("foo bar"))))
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            parse(String::from(">> plop: coucou")),
            vec![vec![Part::Metadata(Metadata {
                key: "plop".to_string(),
                value: "coucou".to_string()
            }),]]
        );
        assert_eq!(
            parse(String::from(">> plop: coucou\nplop")),
            vec![
                vec![Part::Metadata(Metadata {
                    key: "plop".to_string(),
                    value: "coucou".to_string()
                })],
                vec![Part::Text(String::from("plop"))]
            ]
        );
    }

    #[test]
    fn test_space() {
        assert_eq!(space("   "), Ok(("", "   ".to_string())));
        assert_eq!(space("   aa"), Ok(("aa", "   ".to_string())));
        assert_eq!(space("   \taa"), Ok(("aa", "   \t".to_string())));
    }

    #[test]
    fn test_end_line() {
        assert_eq!(end_line("\nfoo"), Ok(("foo", "\n")));
    }

    #[test]
    fn test_word() {
        assert_eq!(word("bar\nfoo"), Ok(("\nfoo", "bar".to_string())));
    }

    /// tests from https://github.com/cooklang/spec/blob/main/tests/canonical.yaml

    #[test]
    fn test_basic_direction() {
        assert_eq!(
            parse(String::from("Add a bit of chilli")),
            vec![vec![Part::Text(String::from("Add a bit of chilli"))]]
        );
    }

    #[test]
    fn test_comments() {
        assert_eq!(
            parse(String::from("-- testing comments")),
            Vec::<Vec<Part>>::new()
        );
    }

    #[test]
    fn test_comments_after_ingredients() {
        assert_eq!(
            parse(String::from(
                "@thyme{2%springs} -- testing comments\n  and some text"
            )),
            vec![
                vec![Part::Ingredient(Ingredient {
                    name: "thyme".to_string(),
                    quantity: "2".to_string(),
                    unit: "springs".to_string()
                })],
                vec![Part::Text("and some text".to_string())]
            ]
        );
    }

    #[test]
    fn test_comments_with_ingredients() {
        assert_eq!(
            parse(String::from(
                "-- testing comments\n        @thyme{2%springs}"
            )),
            vec![vec![Part::Ingredient(Ingredient {
                name: "thyme".to_string(),
                quantity: "2".to_string(),
                unit: "springs".to_string()
            }),]]
        );
    }

    #[test]
    fn test_direction_with_ingrident() {
        assert_eq!(
            parse(String::from(
                "Add @chilli{3%items}, @ginger{10%g} and @milk{1%l}."
            )),
            vec![vec![
                Part::Text("Add".to_string()),
                Part::Ingredient(Ingredient {
                    name: "chilli".to_string(),
                    quantity: "3".to_string(),
                    unit: "items".to_string()
                }),
                Part::Text(",".to_string()),
                Part::Ingredient(Ingredient {
                    name: "ginger".to_string(),
                    quantity: "10".to_string(),
                    unit: "g".to_string()
                }),
                Part::Text("and".to_string()),
                Part::Ingredient(Ingredient {
                    name: "milk".to_string(),
                    quantity: "1".to_string(),
                    unit: "l".to_string()
                }),
                Part::Text(".".to_string())
            ]]
        )
    }

    #[test]
    fn test_equipment_multiple_words() {
        assert_eq!(
            parse(String::from("Fry in #frying pan{}")),
            vec![vec![
                Part::Text("Fry in".to_string()),
                Part::Cookware(Cookware {
                    name: "frying pan".to_string(),
                    ..Default::default()
                })
            ]]
        )
    }

    #[test]
    fn test_equipment_multiple_words_with_leading_number() {
        assert_eq!(
            parse(String::from("Fry in #7-inch nonstick frying pan{ }")),
            vec![vec![
                Part::Text("Fry in".to_string()),
                Part::Cookware(Cookware {
                    name: "7-inch nonstick frying pan".to_string(),
                    ..Default::default()
                })
            ]]
        )
    }

    #[test]
    fn test_equipment_multiple_words_with_spaces() {
        assert_eq!(
            parse(String::from("Fry in #frying pan{ }")),
            vec![vec![
                Part::Text("Fry in".to_string()),
                Part::Cookware(Cookware {
                    name: "frying pan".to_string(),
                    ..Default::default()
                })
            ]]
        )
    }

    #[test]
    fn test_equipment_one_word() {
        assert_eq!(
            parse(String::from("Simmer in #pan for some time")),
            vec![vec![
                Part::Text("Simmer in".to_string()),
                Part::Cookware(Cookware {
                    name: "pan".to_string(),
                    ..Default::default()
                }),
                Part::Text("for some time".to_string())
            ]]
        )
    }

    #[test]
    fn test_ingredient_with_emoji() {
        assert_eq!(
            parse(String::from("Add some @🧂")),
            vec![vec![
                Part::Text("Add some".to_string()),
                Part::Ingredient(Ingredient {
                    name: "🧂".to_string(),
                    quantity: "".to_string(),
                    unit: "".to_string()
                })
            ]]
        )
    }

    #[test]
    fn test_ingrident_explicit_units() {
        assert_eq!(
            parse(String::from("@chilli{3%items}")),
            vec![vec![Part::Ingredient(Ingredient {
                name: "chilli".to_string(),
                quantity: "3".to_string(),
                unit: "items".to_string()
            })]]
        )
    }

    #[test]
    fn test_ingrident_explicit_units_with_spaces() {
        assert_eq!(
            parse(String::from("@chilli{ 3 % items }")),
            vec![vec![Part::Ingredient(Ingredient {
                name: "chilli".to_string(),
                quantity: "3".to_string(),
                unit: "items".to_string()
            })]]
        )
    }

    #[test]
    fn test_full() {
        assert_eq!(
            parse(String::from("
>> source: https://www.gimmesomeoven.com/baked-potato/
>> time required: 1.5 hours
>> course: dinner
-- Don't burn the roux!

Mash @potato{2%kg} until smooth -- alternatively, boil 'em first, then mash 'em, then stick 'em in a stew.
Place @bacon strips{1%kg} on a baking sheet and glaze with @syrup{1/2%tbsp}.
")),
            vec![
                vec![Part::Metadata(Metadata { key: "source".to_string(), value: "https://www.gimmesomeoven.com/baked-potato/".to_string() })],
                vec![Part::Metadata(Metadata { key: "time required".to_string(), value: "1.5 hours".to_string() })],
                vec![Part::Metadata(Metadata { key: "course".to_string(), value: "dinner".to_string() })],
                vec![Part::Text("Mash".to_string()), Part::Ingredient(Ingredient { name: "potato".to_string(), quantity: "2".to_string(), unit: "kg".to_string() }), Part::Text("until smooth".to_string())],
                vec![Part::Text("Place".to_string()), Part::Ingredient(Ingredient { name: "bacon strips".to_string(), quantity: "1".to_string(), unit: "kg".to_string() }), Part::Text("on a baking sheet and glaze with".to_string()),
                Part::Ingredient(Ingredient { name: "syrup".to_string(), quantity: "1/2".to_string(), unit: "tbsp".to_string() }), Part::Text(".".to_string())],
            ]
        )
    }
}
