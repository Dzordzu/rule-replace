pub mod config;

use anyhow::Context;
use clap::Parser;
use config::Config;
use minijinja::Environment;
use std::fs::read_dir;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::{env, io};

pub fn get_project_root() -> io::Result<PathBuf> {
    let path = env::current_dir()?;
    let path_ancestors = path.as_path().ancestors();

    for p in path_ancestors {
        let has_cargo = read_dir(p)?.any(|p| p.unwrap().file_name() == *"Cargo.lock");
        if has_cargo {
            return Ok(PathBuf::from(p));
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Ran out of places to find Cargo.toml",
    ))
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, required = true)]
    file: std::path::PathBuf,

    #[arg(short, long, required = true)]
    config: std::path::PathBuf,

    #[arg(short, long, action)]
    inplace: bool,
}

fn find_last_ident(text: &str) -> String {
    fn find_ident(text: &str) -> String {
        if let Some(first_symbol) = text.chars().position(|c| !c.is_whitespace()) {
            text.split_at(first_symbol).0.to_string()
        } else {
            text.to_string()
        }
    }

    if let Some(newline_pos) = text.chars().rev().position(|c| c == '\n') {
        let splitted = text.split_at(text.len() - newline_pos).1;
        find_ident(splitted)
    } else {
        find_ident(text)
    }
}

#[test]
fn test_find_last_ident() {
    assert_eq!(find_last_ident("a\nb"), "".to_string());
    assert_eq!(find_last_ident("a\n  b"), "  ".to_string());
    assert_eq!(find_last_ident("a\n     b"), "     ".to_string());
    assert_eq!(find_last_ident("a\n\nb"), "".to_string());
    assert_eq!(find_last_ident("a\nb\n "), " ".to_string());
    assert_eq!(find_last_ident("  "), "  ".to_string());
}

fn update_ident(text: &str, ident: &str) -> String {
    let mut lines = text.lines();
    if let Some(first) = lines.next() {
        lines.fold(first.to_string(), |acc, line| acc + "\n" + ident + line)
            + if text.ends_with("\n") { "\n" } else { "" }
    } else {
        text.to_string()
    }
}

fn update_ident_inplace(text: &mut String, ident: &str) {
    *text = update_ident(text, ident);
}

#[test]
fn test_update_ident() {
    assert_eq!(update_ident("a\nb", "  "), "a\n  b");
    assert_eq!(update_ident("a\nb\nc", "   "), "a\n   b\n   c");
    assert_eq!(update_ident("a\nb\nc\n", "   "), "a\n   b\n   c\n");
}

fn handle_jinja(
    config_dir: &std::path::Path,
    template: &std::path::PathBuf,
    values: &std::path::PathBuf,
) -> anyhow::Result<String> {
    let template_path = config_dir.join(template);
    let template_file = std::fs::read_to_string(&template_path)
        .context(format!("Issue with file: {template_path:?}"))?;

    let variables_path = config_dir.join(values);
    let variables: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(&variables_path)
            .context(format!("Issue with file: {variables_path:?}"))?,
    )?;
    let mut env = Environment::new();
    env.add_template("template", &template_file)
        .context(format!("Template: {template:?}"))?;
    let context = minijinja::Value::from_serialize(variables);

    let tmpl = env
        .get_template("template")
        .context(format!("Rendering issue with: {template:?}"))?;
    Ok(tmpl.render(context)?)
}

fn evaluate_rule(
    content: String,
    rule: &config::Rule,
    config_dir: &std::path::Path,
) -> anyhow::Result<String> {
    let mut replacement = match &rule.with {
        config::With::File { file } => {
            let filepath = config_dir.join(file);
            std::fs::read_to_string(&filepath).context(format!("File: {filepath:?}"))?
        }
        config::With::Jinja { template, values } => handle_jinja(config_dir, template, values)?,
        config::With::String { string } => string.clone(),
        config::With::Pattern { pattern } => pattern.clone(),
    };

    Ok(match &rule.replace {
        config::Replace::Text(text) => match rule.which {
            config::Which::First => {
                if rule.keep_spaces {
                    if let Some(pos) = content.find(text) {
                        let ident = find_last_ident(text.split_at(pos).0);
                        update_ident_inplace(&mut replacement, &ident);
                    }
                };
                content.replacen(text, &replacement, 1)
            }
            config::Which::FirstInLine => content.lines().fold(String::new(), |acc, line| {
                if rule.keep_spaces {
                    let ident = find_last_ident(line);
                    update_ident_inplace(&mut replacement, &ident);
                };
                acc + &line.replacen(text, &replacement, 1) + "\n"
            }),
            config::Which::Every => {
                if rule.keep_spaces {
                    // Split by any found occurences
                    let mut splitted = content.split(text);
                    if let Some(first) = splitted.next() {
                        splitted
                            .fold((find_last_ident(first), first.to_string()), |acc, part| {
                                (
                                    find_last_ident(part),
                                    acc.1 + update_ident(&replacement, &acc.0).as_str() + part,
                                )
                            })
                            .1
                    } else {
                        content.to_string()
                    }
                } else {
                    content.replace(text, &replacement)
                }
            }
        },
        config::Replace::Line(pattern) => {
            let ending = if content.ends_with("\n") { "\n" } else { "" };
            let lines = content.lines();

            let mut should_change = true;
            let mut result = lines
                .map(|line| {
                    if should_change && line.contains(pattern) {
                        should_change = matches!(
                            rule.which,
                            config::Which::FirstInLine | config::Which::Every
                        );
                        let ident = find_last_ident(line);
                        let replacement = if rule.keep_spaces {
                            update_ident(&replacement, &ident)
                        } else {
                            replacement.clone()
                        };
                        ident.to_string() + &replacement
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            result.push_str(ending);
            result
        }
        config::Replace::Regex(_) => content,
        config::Replace::LinesBetween(start, end) => {
            let mut result = String::new();
            let mut lines = content.lines();
            while let Some(line) = lines.next() {
                result.push_str(line);
                result.push('\n');
                if line.contains(start) {
                    let ident = find_last_ident(line);
                    for line in lines.by_ref() {
                        if line.contains(end) {
                            result.push_str(&ident);
                            result.push_str(&update_ident(&replacement, &ident));
                            result.push('\n');
                            result.push_str(line);
                            result.push('\n');
                            break;
                        }
                    }
                }
            }
            result
        }
    })
}

fn replace(
    mut content: String,
    config: &Config,
    config_dir: &std::path::Path,
) -> anyhow::Result<String> {
    for rule in &config.rules {
        content = evaluate_rule(content, rule, config_dir)?;
    }
    Ok(content)
}

#[test]
fn example_config() {
    let examples = get_project_root().unwrap().join("example");

    for examples in examples.read_dir().unwrap().filter_map(|entry| {
        entry
            .map(|child| child.path().is_dir().then_some(child.path()))
            .ok()
            .flatten()
    }) {
        let config = examples.join("example.toml");
        let config: Config = toml::from_str(&std::fs::read_to_string(config).unwrap()).unwrap();

        let test_dir = examples.join("tests");

        let max = test_dir.read_dir().unwrap().count() / 2;

        for num in 1..max + 1 {
            let input_file = format!("{num:02}.input.txt");
            let output_file = format!("{num:02}.output.txt");

            let input = std::fs::read_to_string(test_dir.join(input_file)).unwrap();
            let result = std::fs::read_to_string(test_dir.join(output_file)).unwrap();

            assert_eq!(
                replace(input, &config, &examples).unwrap(),
                result,
                "Test {num:02}"
            );
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let file_content = std::fs::read_to_string(&args.file).unwrap();
    let config_content = std::fs::read_to_string(&args.config).unwrap();
    let config: Config = toml::from_str(&config_content).unwrap();

    let result = replace(
        file_content,
        &config,
        &args
            .config
            .parent()
            .map(|x| x.to_path_buf())
            .ok_or(anyhow::anyhow!(
                "Could not retrive parent directory of the config"
            ))?,
    )?;

    if args.inplace {
        std::fs::write(&args.file, result).context("Error with saving to file")?;
    } else {
        print!("{}", result);
    }
    Ok(())
}
