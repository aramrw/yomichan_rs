use crate::zip::*;
use std::path::Path;

#[test]
fn test_import_dictionary() {
    let path = std::path::Path::new("./大辞林第四版.zip");
    //let path = std::path::Path::new("C:\\Users\\arami\\Desktop\\bad.zip");
    //let path = std::path::Path::new("C:\\Users\\arami\\Desktop\\good.zip");
    assert!(path.exists());
    import_dictionary(path).unwrap();
}
