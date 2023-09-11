pub mod ascii;
pub mod circuit;
pub mod gates;
pub mod verilog_parser;

use clap::{builder::PossibleValue, value_parser, Arg, ArgAction, ArgMatches, Command};
use std::fmt;
use std::{collections::HashMap, fmt::Debug, str::FromStr};
use termion::color;
use tfhe::prelude::*;
use tfhe::{FheUint128, FheUint16, FheUint32, FheUint64, FheUint8};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PtxtError {
    #[error("Invalid input")]
    InvalidInput,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PtxtType {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    None,
}

#[derive(Clone)]
pub enum FheType {
    U8(FheUint8),
    U16(FheUint16),
    U32(FheUint32),
    U64(FheUint64),
    U128(FheUint128),
    None,
}

impl FromStr for PtxtType {
    type Err = PtxtError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "None" {
            Ok(PtxtType::None)
        } else if let Ok(value) = u8::from_str(s) {
            Ok(PtxtType::U8(value))
        } else if let Ok(value) = u16::from_str(s) {
            Ok(PtxtType::U16(value))
        } else if let Ok(value) = u32::from_str(s) {
            Ok(PtxtType::U32(value))
        } else if let Ok(value) = u64::from_str(s) {
            Ok(PtxtType::U64(value))
        } else if let Ok(value) = u128::from_str(s) {
            Ok(PtxtType::U128(value))
        } else {
            Err(PtxtError::InvalidInput)
        }
    }
}

impl fmt::Display for PtxtType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PtxtType::Bool(value) => write!(f, "Bool({})", value),
            PtxtType::U8(value) => write!(f, "U8({})", value),
            PtxtType::U16(value) => write!(f, "U16({})", value),
            PtxtType::U32(value) => write!(f, "U32({})", value),
            PtxtType::U64(value) => write!(f, "U64({})", value),
            PtxtType::U128(value) => write!(f, "U128({})", value),
            PtxtType::None => write!(f, "None"),
        }
    }
}

impl FheType {
    pub fn decrypt(&self, client_key: &tfhe::ClientKey) -> PtxtType {
        match self {
            FheType::U8(inner_value) => PtxtType::U8(inner_value.decrypt(client_key)),
            FheType::U16(inner_value) => PtxtType::U16(inner_value.decrypt(client_key)),
            FheType::U32(inner_value) => PtxtType::U32(inner_value.decrypt(client_key)),
            FheType::U64(inner_value) => PtxtType::U64(inner_value.decrypt(client_key)),
            FheType::U128(inner_value) => PtxtType::U128(inner_value.decrypt(client_key)),
            FheType::None => panic!("Decrypt found a None value"),
        }
    }
}

// TODO
// arithmetic -i a 15
// boolean: 1) -i a[0] 1 -i a[1] 0 ...
// boolean: 1) -i aeskey 0 ...

pub fn get_input_wire_map(
    inputs_filename: Option<String>,
    wire_inputs: Vec<Vec<&String>>,
    arithmetic_type: &str,
) -> HashMap<String, PtxtType> {
    if let Some(wire_file_name) = &inputs_filename {
        println!(
            "{}[✓]{} Input wires were provided.",
            color::Fg(color::LightGreen),
            color::Fg(color::Reset)
        );

        verilog_parser::read_input_wires(wire_file_name, arithmetic_type)
    } else if !wire_inputs.is_empty() {
        println!(
            "{}[✓]{} Input wires were provided.",
            color::Fg(color::LightGreen),
            color::Fg(color::Reset)
        );

        // [[wire1, value1], [wire2, value2], [wire3, value3]]
        wire_inputs
            .iter()
            .map(|parts| {
                let ptxt = match arithmetic_type {
                    "bool" => PtxtType::Bool(match parts[1].as_str() {
                        "1" => true,
                        s => s.parse::<bool>().unwrap_or(false),
                    }),
                    "u8" => PtxtType::U8(parts[1].parse().unwrap()),
                    "u16" => PtxtType::U16(parts[1].parse().unwrap()),
                    "u32" => PtxtType::U32(parts[1].parse().unwrap()),
                    "u64" => PtxtType::U64(parts[1].parse().unwrap()),
                    "u128" => PtxtType::U128(parts[1].parse().unwrap()),
                    _ => unreachable!(),
                };
                println!("parts {:?} -> {:?}", parts, ptxt);

                (parts[0].to_string(), ptxt) // (wirename, value)
            })
            .collect::<HashMap<_, _>>()
    } else {
        println!(
            "{}[!]{} No input wires specified, they will be initialized to false.",
            color::Fg(color::LightYellow),
            color::Fg(color::Reset)
        );

        HashMap::new()
    }
}

pub fn parse_args() -> ArgMatches {
    Command::new("HELM")
        .about("HELM: Homomorphic Evaluation with EDA-driven Logic Minimization")
        .arg(
            Arg::new("verilog")
                .long("verilog")
                .short('v')
                .value_name("FILE")
                .help("Verilog input file to evaluate")
                .required(true),
        )
        .arg(
            Arg::new("input-wires")
                .long("input-wires")
                .short('w')
                .num_args(2)
                .action(ArgAction::Append)
                .value_parser(value_parser!(String))
                .value_names(["STRING", "HEX"])
                .help("Input wire values (-i wire1 hex1 -i wire2 hex2 ...)")
                .value_delimiter(',')
                .conflicts_with("input-wires-file")
                .required(false),
        )
        .arg(
            Arg::new("input-wires-file")
                .long("input-wires-file")
                .short('i')
                .value_name("FILE")
                .help("CSV file that contains the input wire values (wire, value)")
                .conflicts_with("input-wires")
                .required(false),
        )
        .arg(
            Arg::new("output-wires-file")
                .long("output-wires-file")
                .short('o')
                .value_name("FILE")
                .help("CSV file to write the output wires (wire, value)")
                .required(false)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("arithmetic")
                .long("arithmetic")
                .short('a')
                .value_name("TYPE")
                .help("Precision for arithmetic mode")
                .value_parser([
                    PossibleValue::new("u8"),
                    PossibleValue::new("u16"),
                    PossibleValue::new("u32"),
                    PossibleValue::new("u64"),
                    PossibleValue::new("u128"),
                ])
                .required(false),
        )
        .arg(
            Arg::new("cycles")
                .long("cycles")
                .short('c')
                .value_name("NUMBER")
                .help("Number of cycles for sequential circuits")
                .required(false)
                .default_value("1")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('p')
                .help("Turn verbose printing on")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .get_matches()
}
