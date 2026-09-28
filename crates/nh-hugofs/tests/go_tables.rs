//! Ports of Go's hugofs tests: fs_test.go, fileinfo_test.go, walk_test.go, glob_test.go,
//! filename_filter_fs_test.go and rootmapping_fs_test.go (over `MemMapFs` and the OS fs).

mod support;

use std::sync::Arc;

use nh_common::glob::filename_filter::FilenameFilter;
use nh_hugofs::afero::{self, Fs, MemMapFs, OsFs};
use nh_hugofs::decorators::new_base_file_decorator;
use nh_hugofs::fileinfo::{FileMeta, FileMetaInfo, sort_dir_entries};
use nh_hugofs::fs::{is_os_fs, new_base_path_fs};
use nh_hugofs::rootmapping_fs::{ComponentPath, RootMapping, RootMappingFs};
use nh_hugofs::walk::{Walkway, WalkwayConfig};
use support::TempDir;

fn write(fs: &dyn Fs, name: &str, content: &str) {
    afero::write_file(fs, name, content.as_bytes(), 0o755).unwrap();
}

fn meta(lang: &str) -> Arc<FileMeta> {
    Arc::new(FileMeta {
        lang: lang.to_string(),
        ..Default::default()
    })
}

fn rm(from: &str, to: &str, lang: &str) -> RootMapping {
    RootMapping::new(from, to, meta(lang))
}

// Go: hugofs/walk_test.go:collectPaths
fn collect_paths(fs: Arc<dyn Fs>, root: &str) -> nh_common::Result<Vec<String>> {
    let mut names = Vec::new();
    let mut wfn = |_path: &str, info: &FileMetaInfo| -> nh_common::Result<()> {
        if info.is_dir() {
            return Ok(());
        }
        names.push(info.meta().path_info.as_ref().unwrap().path().to_string());
        Ok(())
    };
    let mut cfg = WalkwayConfig::new(fs, &mut wfn);
    cfg.root = root.to_string();
    cfg.sort_dir_entries = true;
    cfg.fail_on_not_exist = true;
    let r = Walkway::new(cfg).walk();
    r.map(|_| names)
}

// Go: hugofs/walk_test.go:collectFileinfos
fn collect_fileinfos(fs: Arc<dyn Fs>, root: &str) -> nh_common::Result<Vec<FileMetaInfo>> {
    let mut fis = Vec::new();
    let mut wfn = |_path: &str, info: &FileMetaInfo| -> nh_common::Result<()> {
        fis.push(info.clone());
        Ok(())
    };
    let mut cfg = WalkwayConfig::new(fs, &mut wfn);
    cfg.root = root.to_string();
    cfg.sort_dir_entries = true;
    cfg.fail_on_not_exist = true;
    let r = Walkway::new(cfg).walk();
    r.map(|_| fis)
}

fn dirnames(fs: &dyn Fs, name: &str) -> Vec<String> {
    let mut f = fs.open(name).unwrap();
    let names = f.readdirnames(-1).unwrap();
    f.close().unwrap();
    names
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

// Go: hugofs/fs_test.go:TestIsOsFs
#[test]
fn is_os_fs_table() {
    let tmp = TempDir::new("isosfs");
    assert!(is_os_fs(&OsFs));
    assert!(!is_os_fs(&MemMapFs::default()));
    assert!(!is_os_fs(
        new_base_path_fs(afero::new_mem_map_fs(), "/public").as_ref()
    ));
    assert!(is_os_fs(
        new_base_path_fs(afero::new_os_fs(), tmp.path.to_str().unwrap()).as_ref()
    ));
}

// Go: hugofs/fs_test.go:TestNewDefault
#[test]
fn new_default() {
    let tmp = TempDir::new("newdefault");
    let v = nh_config::default_config_provider::DefaultConfigProvider::new();
    use nh_config::config_provider::Provider;
    v.set(
        "workingDir",
        go_value::Value::from(tmp.path.to_str().unwrap()),
    );
    v.set("publishDir", go_value::Value::from("public"));
    let f = nh_hugofs::fs::new_default(&v);
    assert!(f.source.as_any().is::<OsFs>());
    assert!(is_os_fs(f.working_dir_read_only.as_ref()));
    assert!(is_os_fs(f.source.as_ref()));
    assert!(is_os_fs(f.publish_dir.as_ref()));
    assert!(is_os_fs(f.os.as_ref()));
}

// Go: hugofs/fileinfo_test.go:TestFileMeta
#[test]
fn file_meta() {
    // Merge
    let src = FileMeta {
        filename: "fs1".to_string(),
        ..Default::default()
    };
    let mut dst = FileMeta {
        filename: "fd1".to_string(),
        ..Default::default()
    };
    dst.merge(&src);
    assert_eq!(dst.filename, "fd1");

    // Copy
    let src = FileMeta {
        filename: "fs1".to_string(),
        ..Default::default()
    };
    let dst = FileMeta::copy(Some(&src));
    assert_eq!(dst.filename, src.filename);
}

// Go: hugofs/walk_test.go:TestWalk
#[test]
fn walk() {
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());
    write(fs.as_ref(), "b.txt", "content");
    write(fs.as_ref(), "c.txt", "content");
    write(fs.as_ref(), "a.txt", "content");

    let names = collect_paths(fs, "").unwrap();
    assert_eq!(names, s(&["/a.txt", "/b.txt", "/c.txt"]));
}

// Go: hugofs/walk_test.go:TestWalkRootMappingFs
#[test]
fn walk_root_mapping_fs() {
    let prepare = || -> Arc<dyn Fs> {
        let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());
        let testfile = "test.txt";
        write(fs.as_ref(), &format!("a/b/{testfile}"), "some content");
        write(fs.as_ref(), &format!("c/d/{testfile}"), "some content");
        write(fs.as_ref(), &format!("e/f/{testfile}"), "some content");

        let rms = vec![
            rm("static/b", "e/f", ""),
            rm("static/a", "c/d", ""),
            rm("static/c", "a/b", ""),
        ];
        let rfs = RootMappingFs::new(fs, rms).unwrap();
        new_base_path_fs(rfs, "static")
    };

    // Basic
    let bfs = prepare();
    let names = collect_paths(bfs, "").unwrap();
    assert_eq!(names, s(&["/a/test.txt", "/b/test.txt", "/c/test.txt"]));

    // Para
    let bfs = prepare();
    std::thread::scope(|sc| {
        for _ in 0..8 {
            let bfs = bfs.clone();
            sc.spawn(move || {
                collect_paths(bfs.clone(), "").unwrap();
                let fi = bfs.stat("b/test.txt").unwrap();
                assert!(!fi.meta().filename.is_empty());
            });
        }
    });
}

// Go: hugofs/glob_test.go:TestGlob
#[test]
fn glob() {
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());
    let create = |filename: &str| {
        let dir = go_path::filepath::dir(filename);
        if dir != "." {
            fs.mkdir_all(&dir, 0o777).unwrap();
        }
        afero::write_file(
            fs.as_ref(),
            filename,
            format!("content {filename}").as_bytes(),
            0o777,
        )
        .unwrap();
    };
    let collect = |pattern: &str| -> Vec<String> {
        let mut paths = Vec::new();
        nh_hugofs::glob::glob(fs.clone(), pattern, &mut |fi: &FileMetaInfo| {
            paths.push(fi.meta().path_info.as_ref().unwrap().path().to_string());
            Ok(false)
        })
        .unwrap();
        paths
    };

    create("/root.json");
    create("/jsonfiles/d1.json");
    create("/jsonfiles/d2.json");
    create("/jsonfiles/sub/d3.json");
    create("/jsonfiles/d1.xml");
    create("/a/b/c/e/f.json");
    create("/UPPER/sub/style.css");
    create("/root/UPPER/sub/style.css");

    assert_eq!(collect("/jsonfiles/*.json").len(), 2);
    assert_eq!(collect("/*.json").len(), 1);
    assert_eq!(collect("**.json").len(), 5);
    assert_eq!(collect("**").len(), 8);
    assert_eq!(collect("").len(), 0);
    assert_eq!(collect("jsonfiles/*.json").len(), 2);
    assert_eq!(collect("*.json").len(), 1);
    assert_eq!(collect("**.xml").len(), 1);

    assert_eq!(collect("root/UPPER/sub/style.css").len(), 1);
    assert_eq!(collect("UPPER/sub/style.css").len(), 1);
}

// Go: hugofs/filename_filter_fs_test.go:TestFilenameFilterFs
#[test]
fn filename_filter_fs() {
    let base = "/mybase";
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());
    for letter in ["a", "b", "c"] {
        for i in 1..=3 {
            write(
                fs.as_ref(),
                &format!("{base}/{letter}/my{i}.txt"),
                &format!("some text file for{letter}"),
            );
            write(
                fs.as_ref(),
                &format!("{base}/{letter}/my{i}.json"),
                &format!("some json file for{letter}"),
            );
        }
    }

    let fs = new_base_path_fs(fs, base);
    let filter = FilenameFilter::new(&[], &s(&["/b/**.txt"]))
        .unwrap()
        .unwrap();
    let fs = nh_hugofs::filename_filter_fs::new_filename_filter_fs(fs, base, filter);

    let assert_exists = |filename: &str, should_exist: bool| {
        let filename = go_path::filepath::clean(filename);
        let r1 = fs.stat(&filename);
        let r2 = fs.open(&filename);
        if should_exist {
            assert!(r1.is_ok(), "{filename}");
            r2.unwrap().close().unwrap();
        } else {
            assert!(r1.unwrap_err().is_not_exist());
            assert!(r2.err().unwrap().is_not_exist());
        }
    };

    assert_exists("/a/my1.txt", true);
    assert_exists("/b/my1.txt", false);

    assert_eq!(
        dirnames(fs.as_ref(), "/b"),
        s(&["my1.json", "my2.json", "my3.json"])
    );
    assert_eq!(
        dirnames(fs.as_ref(), "/c"),
        s(&[
            "my1.json", "my1.txt", "my2.json", "my2.txt", "my3.json", "my3.txt"
        ])
    );
}

// Go: hugofs/rootmapping_fs_test.go:TestLanguageRootMapping
#[test]
fn language_root_mapping() {
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());

    write(fs.as_ref(), "content/sv/svdir/main.txt", "main sv");
    write(
        fs.as_ref(),
        "themes/a/mysvblogcontent/sv-f.txt",
        "some sv blog content",
    );
    write(
        fs.as_ref(),
        "themes/a/myenblogcontent/en-f.txt",
        "some en blog content in a",
    );
    write(
        fs.as_ref(),
        "themes/a/mysvblogcontent/d1/sv-d1-f.txt",
        "some sv blog content",
    );
    write(
        fs.as_ref(),
        "themes/a/myenblogcontent/d1/en-d1-f.txt",
        "some en blog content in a",
    );
    write(
        fs.as_ref(),
        "themes/a/myotherenblogcontent/en-f2.txt",
        "some en content",
    );
    write(
        fs.as_ref(),
        "themes/a/mysvdocs/sv-docs.txt",
        "some sv docs content",
    );
    write(
        fs.as_ref(),
        "themes/b/myenblogcontent/en-b-f.txt",
        "some en content",
    );

    let rfs = RootMappingFs::new(
        fs,
        vec![
            rm("content/blog", "themes/a/mysvblogcontent", "sv"),
            rm("content/blog", "themes/a/myenblogcontent", "en"),
            rm("content/blog", "content/sv", "sv"),
            rm("content/blog", "themes/a/myotherenblogcontent", "en"),
            rm("content/docs", "themes/a/mysvdocs", "sv"),
        ],
    )
    .unwrap();

    let collected = collect_paths(rfs.clone(), "content").unwrap();
    assert_eq!(
        collected,
        s(&[
            "/blog/d1/en-d1-f.txt",
            "/blog/d1/sv-d1-f.txt",
            "/blog/en-f.txt",
            "/blog/en-f2.txt",
            "/blog/sv-f.txt",
            "/blog/svdir/main.txt",
            "/docs/sv-docs.txt"
        ])
    );

    let dirs = rfs.mounts("content/blog").unwrap();
    assert_eq!(dirs.len(), 4);
    for dir in &dirs {
        dir.meta().open().unwrap().close().unwrap();
    }

    let mut blog = rfs.open("content/blog").unwrap();
    let fis = blog.read_dir(-1).unwrap();
    for fi in &fis {
        fi.meta().open().unwrap().close().unwrap();
    }
    blog.close().unwrap();

    let get_dirnames = |name: &str, rfs: &Arc<RootMappingFs>| -> Vec<String> {
        let names = dirnames(rfs.as_ref(), name);
        let info = rfs.stat(name).unwrap();
        let mut f2 = info.meta().open().unwrap();
        let names2 = f2.readdirnames(-1).unwrap();
        assert_eq!(names2, names);
        f2.close().unwrap();
        names
    };

    let rfs_en = rfs.filter(&|rm: &RootMapping| rm.meta.lang == "en");
    assert_eq!(
        get_dirnames("content/blog", &rfs_en),
        s(&["d1", "en-f.txt", "en-f2.txt"])
    );

    let rfs_sv = rfs.filter(&|rm: &RootMapping| rm.meta.lang == "sv");
    assert_eq!(
        get_dirnames("content/blog", &rfs_sv),
        s(&["d1", "sv-f.txt", "svdir"])
    );

    // Make sure we have not messed with the original
    assert_eq!(
        get_dirnames("content/blog", &rfs),
        s(&["d1", "sv-f.txt", "en-f.txt", "svdir", "en-f2.txt"])
    );

    assert_eq!(get_dirnames("content", &rfs_sv), s(&["blog", "docs"]));
    assert_eq!(get_dirnames("content", &rfs), s(&["blog", "docs"]));
}

// Go: hugofs/rootmapping_fs_test.go:TestRootMappingFsDirnames
#[test]
fn root_mapping_fs_dirnames() {
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());
    let testfile = "myfile.txt";
    fs.mkdir("f1t", 0o755).unwrap();
    fs.mkdir("f2t", 0o755).unwrap();
    fs.mkdir("f3t", 0o755).unwrap();
    write(fs.as_ref(), &format!("f2t/{testfile}"), "some content");

    let rfs = RootMappingFs::new_from_from_to(
        "",
        fs,
        &[
            "static/bf1",
            "f1t",
            "static/cf2",
            "f2t",
            "static/af3",
            "f3t",
        ],
    )
    .unwrap();

    let fif = rfs.stat(&format!("static/cf2/{testfile}")).unwrap();
    assert_eq!(fif.name(), "myfile.txt");
    assert_eq!(fif.meta().filename, "f2t/myfile.txt");

    assert_eq!(dirnames(rfs.as_ref(), "static"), s(&["af3", "bf1", "cf2"]));
}

// Go: hugofs/rootmapping_fs_test.go:TestRootMappingFsFilename
#[test]
fn root_mapping_fs_filename() {
    let tmp = TempDir::new("root-filename");
    let work_dir = tmp.path.to_str().unwrap();
    let fs = new_base_file_decorator(afero::new_os_fs(), Vec::new());

    let testfilename = format!("{work_dir}/f1t/foo/file.txt");
    fs.mkdir_all(&format!("{work_dir}/f1t/foo"), 0o777).unwrap();
    afero::write_file(fs.as_ref(), &testfilename, b"content", 0o666).unwrap();

    let rfs = RootMappingFs::new_from_from_to(
        work_dir,
        fs,
        &[
            "static/f1",
            &format!("{work_dir}/f1t"),
            "static/f2",
            &format!("{work_dir}/f2t"),
        ],
    )
    .unwrap();

    let fi = rfs.stat("static/f1/foo/file.txt").unwrap();
    assert_eq!(fi.meta().filename, testfilename);
    rfs.stat("static/f1").unwrap();
}

// Go: hugofs/rootmapping_fs_test.go:TestRootMappingFsMount
#[test]
fn root_mapping_fs_mount() {
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());
    let testfile = "test.txt";

    write(
        fs.as_ref(),
        &format!("themes/a/mynoblogcontent/{testfile}"),
        "some no content",
    );
    write(
        fs.as_ref(),
        &format!("themes/a/myenblogcontent/{testfile}"),
        "some en content",
    );
    write(
        fs.as_ref(),
        &format!("themes/a/mysvblogcontent/{testfile}"),
        "some sv content",
    );
    write(
        fs.as_ref(),
        "themes/a/mysvblogcontent/other.txt",
        "some sv content",
    );
    write(fs.as_ref(), "themes/a/singlefiles/no.txt", "no text");
    write(fs.as_ref(), "themes/a/singlefiles/sv.txt", "sv text");

    let bfs = new_base_path_fs(fs, "themes/a");
    let mut single_no = rm("content/singles/p1.md", "singlefiles/no.txt", "no");
    single_no.to_base = "singlefiles".to_string();
    let mut single_sv = rm("content/singles/p1.md", "singlefiles/sv.txt", "sv");
    single_sv.to_base = "singlefiles".to_string();
    let rms = vec![
        // Directories
        rm("content/blog", "mynoblogcontent", "no"),
        rm("content/blog", "myenblogcontent", "en"),
        rm("content/blog", "mysvblogcontent", "sv"),
        // Files
        single_no,
        single_sv,
    ];

    let rfs = RootMappingFs::new(bfs, rms).unwrap();

    let blog = rfs.stat("content/blog").unwrap();
    assert!(blog.is_dir());
    let blogm = blog.meta();
    assert_eq!(blogm.lang, "no"); // First match

    let mut f = blogm.open().unwrap();
    let dirs1 = f.readdirnames(-1).unwrap();
    f.close().unwrap();
    // Union with duplicate dir names filtered.
    assert_eq!(dirs1, s(&["test.txt", "test.txt", "other.txt", "test.txt"]));

    let mut d = rfs.open("content/blog").unwrap();
    let files = d.read_dir(-1).unwrap();
    assert_eq!(files.len(), 4);

    let mut singles_dir = rfs.open("content/singles").unwrap();
    let singles = singles_dir.read_dir(-1).unwrap();
    singles_dir.close().unwrap();
    assert_eq!(singles.len(), 2);
    for (i, lang) in ["no", "sv"].iter().enumerate() {
        assert_eq!(singles[i].meta().lang, *lang);
        assert_eq!(singles[i].name(), "p1.md");
    }

    // Test ReverseLookup.
    // Single file mounts.
    let cp = |path: &str, lang: &str| ComponentPath {
        component: "content".to_string(),
        path: path.to_string(),
        lang: lang.to_string(),
        watch: false,
    };
    assert_eq!(
        rfs.reverse_lookup("singlefiles/no.txt").unwrap(),
        vec![cp("singles/p1.md", "no")]
    );
    assert_eq!(
        rfs.reverse_lookup("singlefiles/sv.txt").unwrap(),
        vec![cp("singles/p1.md", "sv")]
    );
    // File inside directory mount.
    assert_eq!(
        rfs.reverse_lookup("mynoblogcontent/test.txt").unwrap(),
        vec![cp("blog/test.txt", "no")]
    );
}

// Go: hugofs/rootmapping_fs_test.go:TestRootMappingFsMountOverlap
#[test]
fn root_mapping_fs_mount_overlap() {
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());
    write(fs.as_ref(), "da/a.txt", "some no content");
    write(fs.as_ref(), "db/b.txt", "some no content");
    write(fs.as_ref(), "dc/c.txt", "some no content");
    write(fs.as_ref(), "de/e.txt", "some no content");

    let rfs = RootMappingFs::new(
        fs,
        vec![
            rm("static", "da", ""),
            rm("static/b", "db", ""),
            rm("static/b/c", "dc", ""),
            rm("/static/e/", "de", ""),
        ],
    )
    .unwrap();

    assert_eq!(dirnames(rfs.as_ref(), "static"), s(&["a.txt", "b", "e"]));
    assert_eq!(dirnames(rfs.as_ref(), "static/b"), s(&["b.txt", "c"]));
    assert_eq!(dirnames(rfs.as_ref(), "static/b/c"), s(&["c.txt"]));

    let fi = rfs.stat("static/b/b.txt").unwrap();
    assert_eq!(fi.name(), "b.txt");
}

// Go: hugofs/rootmapping_fs_test.go:TestRootMappingFsOs
#[test]
fn root_mapping_fs_os() {
    let tmp = TempDir::new("root-mapping-os");
    let d = tmp.path.to_str().unwrap();
    let fs = new_base_file_decorator(afero::new_os_fs(), Vec::new());

    let testfile = "myfile.txt";
    fs.mkdir(&format!("{d}/f1t"), 0o755).unwrap();
    fs.mkdir(&format!("{d}/f2t"), 0o755).unwrap();
    fs.mkdir(&format!("{d}/f3t"), 0o755).unwrap();

    // Deep structure
    fs.mkdir_all(&format!("{d}/d1/d2/d3/d4/d5"), 0o755).unwrap();
    for i in 1..=3 {
        fs.mkdir_all(&format!("{d}/d1/d2/d3/d4/d4-{i}"), 0o755)
            .unwrap();
        write(
            fs.as_ref(),
            &format!("{d}/d1/d2/d3/f-{i}.txt"),
            "some content",
        );
    }

    write(fs.as_ref(), &format!("{d}/f2t/{testfile}"), "some content");

    let mystatic_dir = format!("{d}/mystatic/a/b/c");
    fs.mkdir_all(&mystatic_dir, 0o755).unwrap();
    write(
        fs.as_ref(),
        &format!("{mystatic_dir}/ms-1.txt"),
        "some content",
    );

    let rfs = RootMappingFs::new_from_from_to(
        d,
        fs,
        &[
            "static/bf1",
            &format!("{d}/f1t"),
            "static/cf2",
            &format!("{d}/f2t"),
            "static/af3",
            &format!("{d}/f3t"),
            "static",
            &format!("{d}/mystatic"),
            "static/a/b/c",
            &format!("{d}/d1/d2/d3"),
            "layouts",
            &format!("{d}/d1"),
        ],
    )
    .unwrap();

    let fif = rfs.stat(&format!("static/cf2/{testfile}")).unwrap();
    assert_eq!(fif.name(), "myfile.txt");

    assert_eq!(
        dirnames(rfs.as_ref(), "static"),
        s(&["a", "af3", "bf1", "cf2"])
    );

    let get_dirnames = |dirname: &str| -> Vec<String> {
        let mut names = dirnames(rfs.as_ref(), dirname);
        names.sort();
        names
    };

    assert_eq!(get_dirnames("static/a/b"), s(&["c"]));
    assert_eq!(
        get_dirnames("static/a/b/c"),
        s(&["d4", "f-1.txt", "f-2.txt", "f-3.txt", "ms-1.txt"])
    );
    assert_eq!(
        get_dirnames("static/a/b/c/d4"),
        s(&["d4-1", "d4-2", "d4-3", "d5"])
    );

    let all = collect_paths(rfs.clone(), "static").unwrap();
    assert_eq!(
        all,
        s(&[
            "/a/b/c/f-1.txt",
            "/a/b/c/f-2.txt",
            "/a/b/c/f-3.txt",
            "/a/b/c/ms-1.txt",
            "/cf2/myfile.txt"
        ])
    );

    let fis = collect_fileinfos(rfs.clone(), "static").unwrap();

    let dirc = fis[3].meta();

    let mut f = dirc.open().unwrap();
    let mut dir_entries = f.read_dir(-1).unwrap();
    f.close().unwrap();
    sort_dir_entries(&mut dir_entries);
    let mut i = 0;
    for fi in &dir_entries {
        if fi.is_dir() || fi.name() == "ms-1.txt" {
            continue;
        }
        i += 1;
        assert_eq!(fi.meta().filename, format!("{d}/d1/d2/d3/f-{i}.txt"));
    }

    rfs.stat("layouts/d2/d3/f-1.txt").unwrap();
    rfs.stat("layouts/d2/d3").unwrap();
}

// Go: hugofs/rootmapping_fs_test.go:TestRootMappingFsOsBase
#[test]
fn root_mapping_fs_os_base() {
    let tmp = TempDir::new("root-mapping-os-base");
    let d = tmp.path.to_str().unwrap();
    let fs = new_base_file_decorator(afero::new_os_fs(), Vec::new());

    // Deep structure
    fs.mkdir_all(&format!("{d}/d1/d2/d3/d4/d5"), 0o755).unwrap();
    for i in 1..=3 {
        fs.mkdir_all(&format!("{d}/d1/d2/d3/d4/d4-{i}"), 0o755)
            .unwrap();
        write(
            fs.as_ref(),
            &format!("{d}/d1/d2/d3/f-{i}.txt"),
            "some content",
        );
    }

    let mystatic_dir = format!("{d}/mystatic/a/b/c");
    fs.mkdir_all(&mystatic_dir, 0o755).unwrap();
    write(
        fs.as_ref(),
        &format!("{mystatic_dir}/ms-1.txt"),
        "some content",
    );

    let bfs = new_base_path_fs(fs, d);

    let rfs = RootMappingFs::new_from_from_to(
        "",
        bfs,
        &["static", "mystatic", "static/a/b/c", "d1/d2/d3"],
    )
    .unwrap();

    let mut names = dirnames(rfs.as_ref(), "static/a/b/c");
    names.sort();
    assert_eq!(
        names,
        s(&["d4", "f-1.txt", "f-2.txt", "f-3.txt", "ms-1.txt"])
    );
}

// Go: hugofs/rootmapping_fs_test.go:TestRootMappingFileFilter
#[test]
fn root_mapping_file_filter() {
    let fs = new_base_file_decorator(afero::new_mem_map_fs(), Vec::new());

    for lang in ["no", "en", "fr"] {
        for i in 1..=3 {
            write(
                fs.as_ref(),
                &format!("{lang}/my{lang}{i}.txt"),
                &format!("some text file for{lang}"),
            );
        }
    }
    for lang in ["no", "en", "fr"] {
        for i in 1..=3 {
            write(
                fs.as_ref(),
                &format!("{lang}/sub/mysub{lang}{i}.txt"),
                &format!("some text file for{lang}"),
            );
        }
    }

    let txt_filter = || FilenameFilter::new(&[], &s(&["**.txt"])).unwrap();
    let rms = vec![
        RootMapping::new(
            "content",
            "no",
            Arc::new(FileMeta {
                lang: "no".to_string(),
                inclusion_filter: txt_filter(),
                ..Default::default()
            }),
        ),
        rm("content", "en", "en"),
        RootMapping::new(
            "content",
            "fr",
            Arc::new(FileMeta {
                lang: "fr".to_string(),
                inclusion_filter: txt_filter(),
                ..Default::default()
            }),
        ),
    ];

    let rfs = RootMappingFs::new(fs, rms).unwrap();

    let assert_exists = |filename: &str, should_exist: bool| {
        let filename = go_path::filepath::clean(filename);
        let r1 = rfs.stat(&filename);
        let r2 = rfs.open(&filename);
        if should_exist {
            r1.unwrap();
            r2.unwrap().close().unwrap();
        } else {
            assert!(r1.is_err());
            assert!(r2.is_err());
        }
    };

    assert_exists("content/myno1.txt", false);
    assert_exists("content/myen1.txt", true);
    assert_exists("content/myfr1.txt", false);

    let dir_entries_sub = afero::read_dir(rfs.as_ref(), "content/sub").unwrap();
    assert_eq!(dir_entries_sub.len(), 3);

    let mut f = rfs.open("content").unwrap();
    let dir_entries = f.read_dir(-1).unwrap();
    f.close().unwrap();
    assert_eq!(dir_entries.len(), 4);
}
