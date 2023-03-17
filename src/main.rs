#![feature(iter_array_chunks)]

use apache_avro::types::Record;
use apache_avro::{Schema, Writer};
use clap::Parser;
use csv::{ReaderBuilder, StringRecord};
use hashbrown::HashMap;
use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use unicode_normalization::UnicodeNormalization;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Wikipedia's page SQL table dump.
    #[arg(long)]
    input_page: String,
    /// Path to the Wikipedia's page props SQL table dump.
    #[arg(long)]
    input_page_props: String,
    /// Path to the Wikipedia's redirect SQL table dump.
    #[arg(long)]
    input_redirect: String,
    /// Path to the Apache Avro file containing the pageid, qid, and title fields.
    #[arg(long)]
    output: String,
}

fn parse_page(page_path: &str) -> HashMap<String, u32> {
    let mut title2pageid = HashMap::new();

    let page = fs::read(page_path).unwrap();
    for line in String::from_utf8_lossy(&page).lines() {
        let pattern_start = "INSERT INTO `page` VALUES ";
        if !line.starts_with(pattern_start) {
            continue;
        }
        let line = &line[pattern_start.len()..(line.len() - 1)];

        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .quote(b'\'')
            .escape(Some(b'\\'))
            .double_quote(false)
            .from_reader(line.as_bytes());

        let fields = &reader
            .records()
            .collect::<Result<Vec<StringRecord>, csv::Error>>()
            .unwrap()[0];

        for record in fields.iter().array_chunks::<12>() {
            let namespace = record[1];

            if namespace != "0" {
                continue;
            }

            let pageid = record[0][1..].parse::<u32>().unwrap();
            let title = record[2].to_string().nfc().collect::<String>();
            title2pageid.insert(title, pageid);
        }
    }

    title2pageid
}

fn parse_page_props(page_props_path: &str) -> BTreeMap<u32, u32> {
    let mut pageid2qid = BTreeMap::new();

    let page_props = fs::read(page_props_path).unwrap();
    for line in String::from_utf8_lossy(&page_props).lines() {
        let pattern_start = "INSERT INTO `page_props` VALUES ";
        if !line.starts_with(pattern_start) {
            continue;
        }
        let line = &line[pattern_start.len()..(line.len() - 1)];

        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .quote(b'\'')
            .escape(Some(b'\\'))
            .double_quote(false)
            .from_reader(line.as_bytes());

        let fields = &reader
            .records()
            .collect::<Result<Vec<StringRecord>, csv::Error>>()
            .unwrap()[0];

        for record in fields.iter().array_chunks::<4>() {
            let property_name = record[1];

            if property_name != "wikibase_item" {
                continue;
            }

            let pageid = record[0][1..].parse::<u32>().unwrap();
            let qid = record[2][1..].parse::<u32>().unwrap();
            pageid2qid.insert(pageid, qid);
        }
    }

    pageid2qid
}

fn parse_redirect(
    redirect_path: &str,
    pageid2qid: &mut BTreeMap<u32, u32>,
    title2pageid: &HashMap<String, u32>,
) {
    let redirect = fs::read(redirect_path).unwrap();
    for line in String::from_utf8_lossy(&redirect).lines() {
        let pattern_start = "INSERT INTO `redirect` VALUES ";
        if !line.starts_with(pattern_start) {
            continue;
        }
        let line = &line[pattern_start.len()..(line.len() - 1)];

        let mut reader = ReaderBuilder::new()
            .has_headers(false)
            .quote(b'\'')
            .escape(Some(b'\\'))
            .double_quote(false)
            .from_reader(line.as_bytes());

        let fields = &reader
            .records()
            .collect::<Result<Vec<StringRecord>, csv::Error>>()
            .unwrap()[0];

        for record in fields.iter().array_chunks::<5>() {
            let namespace = record[1];

            if namespace != "0" {
                continue;
            }

            let source_pageid = record[0][1..].parse::<u32>().unwrap();
            let target_title = record[2].to_string();

            let target_pageid = match title2pageid.get(&target_title) {
                Some(&pageid) => pageid,
                None => continue,
            };
            let target_qid = match pageid2qid.get(&target_pageid) {
                Some(&qid) => qid,
                None => continue,
            };

            pageid2qid.insert(source_pageid, target_qid);
        }
    }
}

fn write_to_disk(
    output_path: &str,
    pageid2qid: &BTreeMap<u32, u32>,
    title2pageid: &HashMap<String, u32>,
) {
    let schema = r#"
    {
        "type": "record",
        "name": "wiki2qid",
        "fields": [
            {"name": "title", "type": "string"},
            {"name": "pageid", "type": "int"},
            {"name": "qid", "type": ["null", "int"]}
        ]
    }
    "#;

    let schema = Schema::parse_str(schema).unwrap();

    let file = File::create(output_path).unwrap();
    let mut writer = Writer::new(&schema, file);

    for (title, &pageid) in title2pageid.into_iter() {
        let qid = match pageid2qid.get(&pageid) {
            Some(&qid) => Some(qid as i32),
            None => None,
        };

        let mut record = Record::new(writer.schema()).unwrap();
        record.put("title", title.to_owned());
        record.put("pageid", pageid as i32);
        record.put("qid", qid);

        writer.append(record).unwrap();
    }
    writer.flush().unwrap();
}

fn main() {
    let args = Args::parse();

    let title2pageid = parse_page(&args.input_page);
    let mut pageid2qid = parse_page_props(&args.input_page_props);
    parse_redirect(&args.input_redirect, &mut pageid2qid, &title2pageid);

    write_to_disk(&args.output, &pageid2qid, &title2pageid);
}
