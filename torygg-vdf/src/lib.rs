#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn parse() {
        let buf = "\
\"basegroup\"
{
    \"key5\" \"value5\"
    \"empty\" \"\"
    \"key3\" \"value3\"
    \"subgroup\"
    {
        \"key1\" \"value1\"
    }
    \"othersubgroup\"
    {
        \"key2\" \"value2\"
    }
    \"key4\" \"value\n4\"
}
";

        let kv = crate::parse(&mut buf.as_bytes()).unwrap();
        assert_eq!(kv[&PathBuf::from("basegroup/key5")], "value5");
        assert_eq!(kv[&PathBuf::from("basegroup/empty")], "");
        assert_eq!(kv[&PathBuf::from("basegroup/key3")], "value3");
        assert_eq!(kv[&PathBuf::from("basegroup/subgroup/key1")], "value1");
        assert_eq!(kv[&PathBuf::from("basegroup/othersubgroup/key2")], "value2");
        assert_eq!(kv[&PathBuf::from("basegroup/key4")], "value\n4");
    }
}

use std::{collections::HashMap, io::Result, path::PathBuf};

/// Parse a buffer.
/// Returns a key / value hashmap.
pub fn parse<T: std::io::Read>(buf: &mut T) -> Result<HashMap<PathBuf, String>> {
    let mut string = String::new();
    buf.read_to_string(&mut string)?;

    let mut in_quotes = false;
    let mut escape = false;
    let mut current_key = String::new();
    let mut current_value = String::new();
    let mut path = PathBuf::new();
    let mut kv_pairs = HashMap::<_, _>::new();
    for char in string.chars() {
        match char {
            '\"' if !escape => {
                if in_quotes {
                    if current_key.is_empty() {
                        std::mem::swap(&mut current_key, &mut current_value);
                    } else {
                        kv_pairs.insert(
                            path.join(std::mem::take(&mut current_key)),
                            std::mem::take(&mut current_value),
                        );
                    }
                }

                in_quotes = !in_quotes;
            }
            '{' if !in_quotes => {
                path.push(&current_key);
                current_key.clear();
            }
            '}' if !in_quotes => {
                path.pop();
            }
            char => {
                if escape {
                    match char {
                        'n' => current_value.push('\n'),
                        't' => current_value.push('\t'),
                        '\\' => current_value.push('\\'),
                        '\"' => current_value.push('\"'),
                        _ => (),
                    }

                    escape = false;
                    continue;
                } else if char == '\\' {
                    escape = true;
                    continue;
                }

                if in_quotes {
                    current_value.push(char);
                }
            }
        }
    }

    Ok(kv_pairs)
}
