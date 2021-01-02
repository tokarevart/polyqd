# polyqd

Polycrystalline cuboidic specimen generation and meshing software.

### Installation

1. Install [Gmsh](https://gmsh.info/) and [Neper](http://www.neper.info/).
   They must be available at the terminal as the commands `gmsh` and `neper` at run time.
2. Install polyqd with `cargo install --git https://github.com/tokarevart/polyqd.git`.
   If you want to install Rust/Cargo, this is probably the easiest way: https://www.rustup.rs.

### Usage
First use `tess` module to generate initial tesselation,
then use `reg` module to regularize that tesselation
and only then mesh regularized tesselation with `mesh` module.

Every time you use a module its last result is cached
in `./polyqd-cache` directory, so, for example, you don't need
to generate and regularize tesselation multiple times to create meshes with
different characteristic length.

### Dimensions

Set them in `--dims` option in the order dx dy dz

### Example

``` sh
$ polyqd tess -n 20 --dims 50 12 4  
$ polyqd reg --fmax 20 --sel 3 --mloop 5  
$ polyqd mesh --cl 3   -o polyqd-rough.msh
$ polyqd mesh --cl 1   -o polyqd.msh2
$ polyqd mesh --cl 0.3 -o polyqd-fine.key
```
