// use std::collections::HashSet;
//
// use crate::{
//     database::dictionary_database::Queries, dictionary_importer::*, settings::Options, Yomichan,
// };
//
// #[test]
// fn dict() {
//     #[cfg(target_os = "linux")]
//     let guard = pprof::ProfilerGuardBuilder::default()
//         .frequency(1000)
//         .blocklist(&["libc", "libgcc", "pthread", "vdso"])
//         .build()
//         .unwrap();
//
//     let options = Options::default();
//     let path = std::path::Path::new("./test_dicts/daijisen");
//     prepare_dictionary(path, &options).unwrap();
//
//     #[cfg(target_os = "linux")]
//     if let Ok(report) = guard.report().build() {
//         let file = std::fs::File::create("flamegraph.svg").unwrap();
//         report.flamegraph(file).unwrap();
//     };
// }
//
// #[test]
// fn init_db() {
//     let db_path = String::from("./a");
//     let mut ycd = Yomichan::new(db_path).unwrap();
//     let paths = ["./test_dicts/daijisen", "./test_dicts/ajdfreq"];
//     ycd.import_dictionaries(&paths).unwrap();
// }
//
// #[test]
// fn query_exact() {
//     let db_path = String::from("./a");
//     let ycd = Yomichan::new(db_path).unwrap();
//
//     let terms = ycd.lookup_exact("亞").unwrap();
//
//     for t in terms {
//         println!("{:#?}", t);
//     }
// }
//
// #[test]
// fn bulk_query() {
//     let db_path = String::from("./a");
//     let ycd = Yomichan::new(db_path).unwrap();
//     //let yomu = Vec::from(["詠む", "読む"]);
//     let nomu = Vec::from([/*"呑む",*/ "読む"]);
//     let terms = ycd.bulk_lookup_term(Queries::Exact(&nomu)).unwrap();
//
//     for t in terms {
//         println!("{:#?}", t);
//     }
// }
//
// #[test]
// fn is_kana() {
//     use crate::database::handlers::is_kana;
//     let kana = "む";
//     assert!(is_kana(kana));
// }
//
// // #[test]
// // fn query_seq() {
// //     let db_path = String::from("./a");
// //     let ycd = Yomichan::new(db_path).unwrap();
// //
// //     // 伏線: 13975000000
// //     // ありがとう: 504000000
// //     let terms = ycd.lookup_seqs(&[16713100000], None).unwrap();
// //     //let terms = ycd.lookup_seq(13975000000).unwrap();
// //
// //     for t in terms {
// //         println!("{:#?}", t);
// //     }
// // }
