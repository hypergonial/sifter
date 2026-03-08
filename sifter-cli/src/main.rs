use std::{fs::File, io::IsTerminal, path::PathBuf, str::FromStr};

use clap::{ArgAction, Parser};
use sifter::Exp;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None, name = "sifter")]
struct Cli {
    exp: String,

    file: Option<VarValue>,

    #[arg(short, long, value_name = "VAR", num_args = 0.., action = ArgAction::Append)]
    var: Vec<Var>,

    #[arg(long, action = ArgAction::SetTrue)]
    debug: bool,
}

#[derive(Debug, Clone)]
struct Var {
    name: Box<str>,
    value: VarValue,
}

impl FromStr for Var {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, '=');
        let name = parts
            .next()
            .ok_or_else(|| "Missing variable name".to_string())?
            .trim()
            .into();
        let value_str = parts
            .next()
            .ok_or_else(|| "Missing variable value".to_string())?
            .trim();
        let value = value_str.parse()?;
        Ok(Self { name, value })
    }
}

#[derive(Debug, Clone)]
enum VarValue {
    Json(serde_json::Value),
    Path(PathBuf),
}

impl FromStr for VarValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try parsing as serde json value, fall back to path if it fails
        serde_json::from_str(s)
            .map(Self::Json)
            .or_else(|_| Ok(Self::Path(PathBuf::from(s))))
    }
}

impl TryFrom<VarValue> for serde_json::Value {
    type Error = String;

    fn try_from(value: VarValue) -> Result<Self, Self::Error> {
        match value {
            VarValue::Path(path) => {
                serde_json::from_reader(File::open(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())
            }
            VarValue::Json(json) => Ok(json),
        }
    }
}

fn vars_into_bindmap(vars: Vec<Var>) -> impl Iterator<Item = (Box<str>, serde_json::Value)> {
    vars.into_iter().map(|var| {
        let value = var.value.try_into().unwrap_or_else(|e| {
            eprintln!("Error loading variable '{}': {e}", var.name);
            std::process::exit(1);
        });
        (var.name, value)
    })
}

fn main() {
    let args = Cli::parse();
    let exp = Exp::new(args.exp.as_str()).unwrap_or_else(|e| {
        eprintln!("Error parsing expression: {e}");
        std::process::exit(1);
    });

    let input: Option<serde_json::Value> = match args.file {
        Some(var) => Some(var.try_into().unwrap_or_else(|e| {
            eprintln!("Error loading variable from file: {e}");
            std::process::exit(1);
        })),
        None => {
            if std::io::stdin().is_terminal() {
                None
            } else {
                serde_json::from_reader(std::io::stdin())
                    .map_err(|e| {
                        if args.var.is_empty() {
                            eprintln!("Error reading JSON from stdin: {e}");
                            std::process::exit(1);
                        } else {
                            e
                        }
                    })
                    .ok()
            }
        }
    };
    let mut env = sifter::Env::new();

    env.bind_multiple(vars_into_bindmap(args.var));
    if let Some(input) = input.as_ref() {
        env.bind_ref("value", input)
            .bind_ref("input", input)
            .bind_ref("it", input);
    }
    let env = env.build();

    if args.debug {
        eprintln!("AST {exp:#?}");
        eprintln!("--------------");
        eprintln!("{env:#?}");
        eprintln!("--------------");
    }

    let result = exp.eval(&env).unwrap_or_else(|e| {
        eprintln!("Error evaluating expression: {e}");
        std::process::exit(1);
    });
    println!("{result}");
}
