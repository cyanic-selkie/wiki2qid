<div align="center">
    <h1>wiki2qid</h1>
    <p>
    A program for efficiently generating mappings between Wikipedia's titles, Wikipedia's page IDs, and Wikidata's QIDs.
    </p>
</div>
<p align="center">
    <a href="https://crates.io/crates/wiki2qid">
        <img alt="Release" src="https://img.shields.io/crates/v/wiki2qid">
    </a>
    <a href="https://docs.rs/wiki2qid">
        <img alt="Docs" src="https://img.shields.io/docsrs/wiki2qid">
    </a>
    <a href="https://github.com/cyanic-selkie/wiki2qid/blob/main/LICENSE">
        <img alt="License" src="https://img.shields.io/crates/l/wiki2qid">
    </a>
    <img alt="Downloads" src="https://shields.io/crates/d/wiki2qid">
</p>

This is effectively a reimplementation of the [`wikimapper`](https://github.com/jcklie/wikimapper) library in Rust. A major difference is that `wiki2qid` generates an [Apache Avro](https://avro.apache.org/) file instead of a SQLite file. The reason for this is the ability to efficiently insert the mapping data into any data structure you want. Incidentally, this also means that `wiki2qid` does not support querying the mappings.

**Note:** Some Wikipedia pages are redirects and therefore map to the same QID (i.e., it is a many-to-one relationship).

**Note:** Some Wikipedia pages don't have corresponding Wikidata items (i.e., their QIDs are null).

## Usage

You can install `wiki2qid` by running the following command:

```bash
cargo install wiki2qid
```

Of course, you can also build it from source.

`wiki2qid` requires 3 files as input. They are the [page](https://www.mediawiki.org/wiki/Manual:Page_table), [page_props](https://www.mediawiki.org/wiki/Manual:Page_props_table), and [redirect](https://www.mediawiki.org/wiki/Manual:Redirect_table) SQL table dumps. You can download them with the following commands:

```
wget https://dumps.wikimedia.org/${LANGUAGE}wiki/latest/${LANGUAGE}wiki-latest-page.sql.gz
wget https://dumps.wikimedia.org/${LANGUAGE}wiki/latest/${LANGUAGE}wiki-latest-page_props.sql.gz
wget https://dumps.wikimedia.org/${LANGUAGE}wiki/latest/${LANGUAGE}wiki-latest-redirect.sql.gz
```

Replace the `${LANGUAGE}` with two letter language codes (e.g., "en", "hr").

After decompressing the SQL table dumps, you can extract the mapping data with the following command:
```bash
wiki2qid --input-page "${LANGUAGE}wiki-latest-page.sql" \
         --input-page_props "${LANGUAGE}wiki-latest-page_props.sql" \
         --input-redirect "${LANGUAGE}wiki-latest-redirect.sql" \
         --output wiki2qid.avro
```

The schema of the output is defined by the following JSON:

```json
{
    "type": "record",
    "name": "wiki2qid",
    "fields": [
        {"name": "title", "type": "string"},
        {"name": "pageid", "type": "int"},
        {"name": "qid", "type": ["null", "int"]}
    ]
}
```

### Helper Scripts

The help with this, there are 2 helper scripts in the `helpers/` directory.

You can use them by first downloading and decompressing the data with the following command:

```bash
./download --download-dir ${DOWNLOAD_DIR} --language ${LANGUAGE_1} --language ${LANGUAGE_2}
```

You can pass in any number of languages.

After you've done that, you can generate the mappings with the following command:

```bash
./generate --download-dir ${DOWNLOAD_DIR} --output-dir ${OUTPUT_DIR} --output-filename ${OUTPUT_FILENAME} --language ${LANGUAGE_1} --language ${LANGUAGE_2}
```

The argument `--output-filename` is optional and has the default value of `wiki2qid.avro`.

You can find the mapping data here: `${OUTPUT_DIR}/${LANGUAGE_i}/${OUTPUT_FILENAME}`.

## Performance

`wiki2qid` uses a single thread. On the English dump from March 2023, containing \~6,600,000 articles, it takes \~1.5 minutes to complete with peak memory usage of \~11GB on an AMD Ryzen Threadripper 3970X CPU and an SSD.
