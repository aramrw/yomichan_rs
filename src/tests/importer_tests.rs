use crate::dictionary_importer::*;
#[allow(unused_imports)]
use crate::structured_content::ContentMatchType;

#[test]
fn dict() {
    let path = std::path::Path::new("./大辞林第四版");
    prepare_dictionary(path).unwrap();
}

