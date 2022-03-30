use geoconv::*;
use std::fs;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use clap::{App, Arg, ArgMatches};
use indoc::indoc;

const CACHE_DIR: &str = "polyqd-cache";
fn rel_cache(filename: &str) -> String {
    format!("{}/{}", CACHE_DIR, filename)
}

mod generic_tess {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct Tessellation {
        pub n: String,
        pub morpho: String,
        pub domain: Option<String>,
        pub morphooptiini: Option<String>,
        pub reg: Option<String>,
        pub fmax: Option<String>,
        pub sel: Option<String>,
        pub mloop: Option<String>,
        pub output: Option<String>,
        pub format: Option<String>,
    }

    impl Tessellation {
        pub fn new(n: &str) -> Self {
            Self {
                n: n.into(),
                morpho: "".to_owned(),
                domain: None,
                morphooptiini: None,
                reg: None,
                fmax: None,
                sel: None,
                mloop: None,
                output: None,
                format: None,
            }
        }

        pub fn domain(&mut self, v: &str) -> &mut Self  {
            self.domain = Some(v.into());
            self
        }

        pub fn morpho(&mut self, v: &str) -> &mut Self  {
            self.morpho = v.into();
            self
        }

        pub fn morphooptiini(&mut self, v: &str) -> &mut Self  {
            self.morphooptiini = Some(v.into());
            self
        }

        pub fn reg(&mut self, v: &str) -> &mut Self {
            if v != "0" && v != "1" {
                panic!();
            }
            self.reg = Some(v.into());
            self
        }

        pub fn fmax(&mut self, v: &str) -> &mut Self {
            self.fmax = Some(v.into());
            self
        }

        pub fn sel(&mut self, v: &str) -> &mut Self {
            self.sel = Some(v.into());
            self
        }

        pub fn mloop(&mut self, v: &str) -> &mut Self {
            self.mloop = Some(v.into());
            self
        }

        pub fn output(&mut self, v: &str) -> &mut Self  {
            self.output = Some(v.into());
            self
        }

        pub fn format(&mut self, v: &str) -> &mut Self {
            self.format = Some(v.into());
            self
        }

        pub fn run(&self) {
            let mut args = vec!["-T".to_owned()];
            let mut ext_args = |x: &str, y: &Option<String>| {
                if let Some(v) = y {
                    args.extend_from_slice(&[x.to_owned(), v.to_owned()]);
                }
            };
            ext_args("-n", &Some(self.n.clone()));
            ext_args("-domain", &self.domain);
            ext_args("-morpho", &Some(self.morpho.clone()));
            ext_args("-morphooptiini", &self.morphooptiini);
            ext_args("-reg", &self.reg);
            ext_args("-fmax", &self.fmax);
            ext_args("-sel", &self.sel);
            ext_args("-mloop", &self.mloop);
            ext_args("-o", &self.output);
            ext_args("-format", &self.format);
            Command::new("neper")
                    .args(["--rcfile", "none"].iter())
                    .args(args)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .unwrap();
        }
    }
}
use generic_tess::*;

#[derive(Debug, Clone)]
pub struct Tess {
    tess: Tessellation, 
    dims: SpecDims, 
    n: String,
}

impl Tess {
    pub fn new(config: Config) -> Self {
        let Config{ dims, n, morpho } = config;
        let mut tess = Tessellation::new(&n);
        let domain = format!(
            "cube({},{},{})", 
            dims.dx, dims.dy, dims.dz,
        );
        tess.morpho(&morpho)
            .domain(&domain)
            .output(&rel_cache("polyqd-tess"))
            .format("tess");
        Self{ tess, dims, n }
    }

    pub fn run(&self) {
        Config{ 
            dims: self.dims, 
            n: self.n.clone(), 
            morpho: self.tess.morpho.clone() 
        }.serialize_to_file();
        self.tess.run()
    }
}

#[derive(Debug, Clone)]
pub struct Reg(Tessellation);

impl Reg {
    pub fn new() -> Self {
        let Config{ dims, n, morpho } = Config::deserialize_from_file();
        let mut tess = Tessellation::new(&n);
        let domain = format!(
            "cube({},{},{})", 
            dims.dx, dims.dy, dims.dz,
        );
        tess.reg("1")
            .morphooptiini(&format!("coo:file({}),weight:file({})", 
                                    rel_cache("polyqd-tess.tess"), 
                                    rel_cache("polyqd-tess.tess")))
            .domain(&domain)
            .morpho(&morpho)
            .output(&rel_cache("polyqd-tess-reg"))
            .format("geo");
        Self(tess)
    }

    pub fn morpho(&mut self, v: &str) -> &mut Self  {
        self.0.morpho(v);
        self
    }

    pub fn fmax(&mut self, v: &str) -> &mut Self {
        self.0.fmax(v);
        self
    }

    pub fn sel(&mut self, v: &str) -> &mut Self {
        self.0.sel(v);
        self
    }

    pub fn mloop(&mut self, v: &str) -> &mut Self {
        self.0.mloop(v);
        self
    }

    pub fn run(&self) {
        self.0.run();
        Self::convert_geo();
    }

    fn convert_geo() {
        let file = GeoFile::open(&rel_cache("polyqd-tess-reg.geo")).unwrap();
        let mut geom = Geometry::from(file);
        geom.clear(GeoElemKind::PhysicalSurface);
        let stags: Vec<u64> = geom.tags(GeoElemKind::Surface).map(|x| *x).collect();
        for stag in stags {
            geom.correct_surface_flatness(stag).unwrap();
        }
        let mut file = OccFile::create(&rel_cache("polyqd.geo")).unwrap();
        file.write_geometry(&geom).unwrap();
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    script: String,
}

impl Mesh {
    pub fn new(cl: &str, output: &str) -> Self {
        let script = [
            format!("var_cl = {};", cl),
            include_str!("../gmsh-script-part.geo").to_owned(),
            format!("Save \"../{}\";", output),
        ].join("\n");
        Self { script }
    }

    pub fn run(&self) {
        fs::create_dir_all(CACHE_DIR).unwrap();
        fs::write(&rel_cache("script.geo"), &self.script).unwrap();
        let script_path = rel_cache("script.geo");
        let args: Vec<&str> = vec![&script_path, "-"];
        Command::new("gmsh").args(args)
                            .stdout(Stdio::inherit())
                            .stderr(Stdio::inherit())
                            .output()
                            .unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct SpecDims {
    pub dx: f64, 
    pub dy: f64, 
    pub dz: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub dims: SpecDims,
    pub n: String,
    pub morpho: String,
}

impl Config {
    pub fn serialize_to_file(&self) {
        let ser = serde_json::to_string(self).unwrap();
        fs::create_dir_all(CACHE_DIR).unwrap();
        fs::write(&rel_cache("config.json"), ser).unwrap();
    }

    pub fn deserialize_from_file() -> Self {
        let de = fs::read_to_string(&rel_cache("config.json")).unwrap();
        serde_json::from_str(&de).unwrap()
    }
}

fn cli() -> ArgMatches {
    let default_fmax = "20";
    let default_mloop = "5";

    App::new("polyqd")
        .author("Tokarev Artyom <tokarev28.art@gmail.com>")
        .about("Polycrystalline cuboidic specimen generation and meshing software")
        .after_help(indoc!("\
            First use tess module to generate initial tesselation,
            then use reg module to regularize that tesselation
            and only then mesh regularized tesselation with mesh module.

            Every time you use a module its last result is cached
            in ./polyqd-cache directory, so, for example, you don't need
            to generate and regularize tesselation multiple time to create meshes with
            different characteristic lengths."))
        .subcommand(App::new("tess")
            .about("Generates cuboidic specimen as tessellation")
            .arg(Arg::new("n")
                .short('n')
                .required(true)
                .takes_value(true)
                .help("Number of grains in granular part of the specimen"))
            .arg(Arg::new("dims")
                .long("dims")
                .required(true)
                .takes_value(true)
                .number_of_values(3)
                .help("Specimen dimensions in the order dx dy dz"))
            .arg(Arg::new("morpho")
                .long("morpho")
                .takes_value(true)
                .help("Morphological properties of the cells")))
        .subcommand(App::new("reg")
            .about("Regularizes a tessellation, that is, removes the small edges and, \
                    indirectly, the small faces")
            .arg(Arg::new("fmax")
                .long("fmax")
                .takes_value(true)
                .default_value(default_fmax)
                .help("Maximum allowed face flatness fault (in degrees)"))
            .arg(Arg::new("sel")
                .long("sel")
                .takes_value(true)
                .help("Absolute small edge (maximum) length"))
            .arg(Arg::new("mloop")
                .long("mloop")
                .takes_value(true)
                .default_value(default_mloop)
                .help("Maximum number of regularization loops")))
        .subcommand(App::new("mesh")
            .about("Meshes regularized tessellation")
            .arg(Arg::new("cl")
                .long("cl")
                .required(true)
                .takes_value(true)
                .help("Absolute characteristic length of the elements"))
            .arg(Arg::new("output")
                .short('o')
                .long("output")
                .required(true)
                .takes_value(true)
                .help("Output file name")))
        .subcommand(App::new("regmesh")
            .about("reg and mesh modules combined, with 'sel' arg equal to 'cl'")
            .arg(Arg::new("fmax")
                .long("fmax")
                .takes_value(true)
                .default_value(default_fmax)
                .help("Maximum allowed face flatness fault (in degrees)"))
            .arg(Arg::new("mloop")
                .long("mloop")
                .takes_value(true)
                .default_value(default_mloop)
                .help("Maximum number of regularization loops"))
            .arg(Arg::new("cl")
                .long("cl")
                .required(true)
                .takes_value(true)
                .help("Absolute characteristic length of the elements"))
            .arg(Arg::new("output")
                .short('o')
                .long("output")
                .required(true)
                .takes_value(true)
                .help("Output file name")))
    .get_matches()
}

fn main() {
    let matches = cli();
    
    if let Some(matches) = matches.subcommand_matches("tess") {
        let n = matches.value_of("n").unwrap().to_owned();
        let dims: Vec<f64> = matches.values_of("dims").unwrap()
                                    .map(|x| x.parse().unwrap())
                                    .collect();
        let dims = SpecDims{ 
            dx: dims[0], 
            dy: dims[1], 
            dz: dims[2], 
        };
        let config = if let Some(v) = matches.value_of("morpho") {
            Config{ dims, n, morpho: v.to_owned() }
        } else {
            Config{ dims, n, morpho: "graingrowth".to_owned() }
        };
        Tess::new(config).run();
    }

    if let Some(matches) = matches.subcommand_matches("reg") {
        let mut reg = Reg::new();
        if let Some(v) = matches.value_of("fmax") {
            reg.fmax(v);
        }
        if let Some(v) = matches.value_of("sel") {
            reg.sel(v);
        }
        if let Some(v) = matches.value_of("mloop") {
            reg.mloop(v);
        }
        reg.run();
    }

    if let Some(matches) = matches.subcommand_matches("mesh") {
        let cl = matches.value_of("cl").unwrap();
        let output = matches.value_of("output").unwrap();
        Mesh::new(cl, output).run();
    }

    if let Some(matches) = matches.subcommand_matches("regmesh") {
        let mut reg = Reg::new();
        if let Some(v) = matches.value_of("fmax") {
            reg.fmax(v);
        }
        if let Some(v) = matches.value_of("mloop") {
            reg.mloop(v);
        }
        let cl = matches.value_of("cl").unwrap();
        reg.sel(cl);
        reg.run();
        let output = matches.value_of("output").unwrap();
        Mesh::new(cl, output).run();
    }
}
