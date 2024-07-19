use crate::{dictionary_importer::*, settings::Options, Yomichan};

#[test]
fn dict() {
    #[cfg(target_os = "linux")]
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    let mut options = Options::default();
    let path = std::path::Path::new("./test_dicts/daijisen");
    prepare_dictionary(path, &mut options).unwrap();

    #[cfg(target_os = "linux")]
    if let Ok(report) = guard.report().build() {
        let file = std::fs::File::create("flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

#[test]
fn token() {
    use lindera::{
        DictionaryConfig, DictionaryKind, LinderaResult, Mode, Tokenizer, TokenizerConfig,
    };

    let dictionary = DictionaryConfig {
        kind: Some(DictionaryKind::UniDic),
        path: None,
    };

    let config = TokenizerConfig {
        dictionary,
        user_dictionary: None,
        mode: Mode::Normal,
    };

    let tokenizer = Tokenizer::from_config(config).unwrap();
    let tokens = tokenizer.tokenize("俺は疲れている").unwrap();

    for token in tokens {
        println!("{}", token.text);
    }
}

#[test]
fn init_db() {
    let db_path = String::from("./test.yc");
    let mut ycd = Yomichan::new(db_path).unwrap();
    let path = std::path::Path::new("./test_dicts/daijisen");
    ycd.import_dictionary(path).unwrap();
}

#[test]
fn query_db() {
    let db_path = String::from("./test.yc");
    let ycd = Yomichan::new(db_path).unwrap();

    let terms = ycd.bulk_lookup("国外").unwrap();

    for t in terms {
        println!("{:#?}", t);
    }
}
