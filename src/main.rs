#[macro_use]
extern crate clap;
extern crate dirs;
extern crate serde_json;
use clap::App;
use serde_json::{ Map, Value };
use serde_json::json;
use std::boxed::Box;
use std::collections::HashMap;
use std::error::Error;
use std::fs::*;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::process::exit;
use std::result::Result;
use std::string::String;

fn exec_command(command: &str) -> Result<(), Box<dyn Error>> {
  let config = load_betterops_config()?;

  match config.get("current") {
    Some(x) if !x.is_null() => { println!("Running with betterops profile: {}", x.as_str().unwrap()); },
    _ => {
      println!("No profile currently set!");
      exit(1);
    },
  };

  let current_profile = config.get("current")
    .unwrap()
    .as_str()
    .unwrap();
  let profiles = config.get("profiles")
    .unwrap();
  let profile_props = profiles.get(current_profile)
    .unwrap()
    .as_array()
    .unwrap();

  let mut profile_envs = HashMap::new();
  for prop in profile_props {
    let prop_key = prop.get("key").unwrap().as_str().unwrap();
    let prop_type = prop.get("type").unwrap().as_str().unwrap();
    let prop_value = prop.get("value").unwrap().as_str().unwrap();

    let mut value = prop_value.to_string();
    if prop_type.trim() == "command" {
      let output = Command::new("bash")
        .envs(&profile_envs)
        .arg("-c")
        .arg(prop_value.to_string())
        .output()
        .unwrap_or_else(|_| panic!("Could not execute command for {}", prop_key.to_string()));
      value = String::from_utf8(output.stdout)?.trim().to_string();
    }
    profile_envs.insert(prop_key.to_string(), value);
  }

  let status = Command::new("bash")
    .envs(&profile_envs)
    .arg("-c")
    .arg(command.to_string())
    .status()
    .expect("Failed to execute command!");

  match status.code() {
    Some(v) => { exit(v); },
    None => { exit(1); },
  };
}

fn configure_profile() -> Result<(), Box<dyn Error>> {
  let mut config = load_betterops_config()?;

  let mut profile_name = String::new();
  while profile_name.trim().is_empty() {
    println!("Name your new profile. Warning: Providing an existing name will override that profile!");
    io::stdin().read_line(&mut profile_name)?;
  }
  config.insert("current".to_string(), Value::from(profile_name.trim()));

  let mut new_profile_map = Map::new();
  new_profile_map.append(config["profiles"].as_object_mut().unwrap());
  let mut new_profile_props = Vec::new();
  let mut has_more_profile_props = "y".to_string();
  while has_more_profile_props.trim() == "y" {
    has_more_profile_props = "".to_string();
    let mut new_prop_map = Map::new();

    let mut new_prop_key = String::new();
    while new_prop_key.trim().is_empty() {
      println!("Provide the ENV key: ");
      io::stdin().read_line(&mut new_prop_key)?;
    }
    new_prop_map.insert("key".to_string(), serde_json::to_value(new_prop_key.trim())?);

    let mut new_prop_type = String::new();
    while new_prop_type.trim() != "command" && new_prop_type.trim() != "value" {
      new_prop_type = String::new();
      println!("Indicate if it is a command or value type [value/command]: ");
      io::stdin().read_line(&mut new_prop_type)?;
    }
    new_prop_map.insert("type".to_string(), serde_json::to_value(new_prop_type.trim())?);

    let mut new_prop_value = String::new();
    while new_prop_value.trim().is_empty() {
      println!("Provide the ENV value or command: ");
      io::stdin().read_line(&mut new_prop_value)?;
    }
    new_prop_map.insert("value".to_string(), serde_json::to_value(new_prop_value.trim())?);

    new_profile_props.push(new_prop_map);
    println!("Attach another ENV to this profile [y/N]?: ");
    io::stdin().read_line(&mut has_more_profile_props)?;
  }

  new_profile_map.insert(profile_name.trim().to_string(), serde_json::to_value(new_profile_props)?);
  config.insert("profiles".to_string(), serde_json::to_value(new_profile_map)?);
  save_betterops_config(&config)?;
  Ok(())
}

fn get_profile() -> Result<(), Box<dyn Error>> {
  let config = load_betterops_config()?;
  match config.get("current") {
    Some(x) if !x.is_null() => { println!("{}", x.as_str().unwrap()); },
    _ => { println!("No profile currently set!"); },
  }
  Ok(())
}

fn list_profile() -> Result<(), Box<dyn Error>> {
  let config = load_betterops_config()?;
  let current_profile = config.get("current").unwrap();
  if current_profile.is_null() {
    println!("There are no profiles!");
    exit(1);
  }

  let profile_map = config.get("profiles")
    .unwrap()
    .as_object()
    .unwrap();
  for (profile_name, _) in profile_map {
    println!("{}", profile_name);
  }
  Ok(())
}

fn set_profile() -> Result<(), Box<dyn Error>> {
  let mut config = load_betterops_config()?;
  let profile_map = config.get("profiles")
    .unwrap()
    .as_object()
    .unwrap();

  let mut selected_profile = String::new();
  while selected_profile.trim().is_empty() {
    println!("Which profile do you want to use?");
    println!("Available profiles");
    for (profile_name, _) in profile_map {
      println!("  {}", profile_name.trim());
    }

    io::stdin().read_line(&mut selected_profile)?;
    if !profile_map.contains_key(selected_profile.trim()) {
      selected_profile = String::new();
    }
  }

  config.insert("current".to_string(), Value::from(selected_profile.trim()));
  save_betterops_config(&config)?;
  Ok(())
}

fn get_betterops_config_path() -> PathBuf {
  dirs::home_dir()
    .unwrap()
    .join(".betterops")
}

fn initialize_betterops_config() -> Result<(), Box<dyn Error>> {
  let init_config = json!({
    "current": null,
    "profiles": {}
  });
  write(get_betterops_config_path(), init_config.to_string())?;
  Ok(())
}

fn load_betterops_config() -> Result<Map<String, Value>, Box<dyn Error>> {
  let byte_content = read(get_betterops_config_path())?;
  let string_content = String::from_utf8(byte_content)?;
  let content = serde_json::from_str(&string_content)?;
  Ok(content)
}

fn save_betterops_config(content: &Map<String, Value>) -> Result<(), Box<dyn Error>> {
  let string_content = serde_json::to_string(content)?;
  write(get_betterops_config_path(), string_content)?;
  Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
  if !get_betterops_config_path().exists() {
    initialize_betterops_config()?;
  }

  let yaml = load_yaml!("constants/cli.yml");
  let matches = App::from_yaml(&yaml["app"]).get_matches();
  
  match matches.subcommand() {
    ("exec", Some(matches)) => {
      exec_command(&matches.values_of("subcommand").unwrap().collect::<Vec<_>>().join(" "))?;
    },
    ("profile", Some(profile_matches)) => {
      match profile_matches.subcommand() {
        ("configure", Some(_)) => {
          configure_profile()?;
        },
        ("get", Some(_)) => {
          get_profile()?;
        },
        ("list", Some(_)) => {
          list_profile()?;
        },
        ("set", Some(_)) => {
          set_profile()?;
        },
        _ => unreachable!(),
      }
    },
    ("", None) => println!("No subcommand was used"),
    _ => unreachable!(),
  }

  Ok(())
}
