<h1 align="center">
  <img
    src="https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/misc/transparent.png"
    height="30"
    width="0px"
    alt=""
  />
  🌳 Terebinth
  <img
    src="https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/misc/transparent.png"
    height="30"
    width="0px"
    alt=""
  />
</h1>

<p align="center">
  <a href="https://github.com/crhowell3/terebinth/stargazers">
    <img
      alt="Stargazers"
      src="https://img.shields.io/github/stars/crhowell3/terebinth?style=for-the-badge&logo=starship&color=b16286&logoColor=d9e0ee&labelColor=282a36"
    />
  </a>
  <a href="https://crates.io/crates/terebinth">
    <img
      alt="Crates.io Version"
      src="https://img.shields.io/crates/v/terebinth?style=for-the-badge&logo=rust&color=458588&logoColor=d9e0ee&labelColor=282a36"
    />
  </a>
  <a href="https://github.com/crhowell3/terebinth/issues">
    <img
      alt="Issues"
      src="https://img.shields.io/github/issues/crhowell3/terebinth?style=for-the-badge&logo=gitbook&color=d79921&logoColor=d9e0ee&labelColor=282a36"
    />
  </a>
  <a href="https://github.com/crhowell3/terebinth/contributors">
    <img
      alt="Contributors"
      src="https://img.shields.io/github/contributors/crhowell3/terebinth?style=for-the-badge&logo=opensourceinitiative&color=689d6a&logoColor=d9e0ee&labelColor=282a36"
    />
  </a>
  <br/>
  <a href="#">
    <img
      alt="Documentation"
      src="https://img.shields.io/docsrs/terebinth?style=for-the-badge&logo=docsdotrs&logoColor=d9e0ee&labelColor=282a36"
    />
  </a>
  <a href="#">
    <img
      alt="Maintained"
      src="https://img.shields.io/maintenance/yes/2025?style=for-the-badge&color=98971a&labelColor=282a36"
    />
  </a>
</p>

&nbsp;

## 💭 About

The Terebinth programming language is a compiled language. The compiler is
built entirely using Rust to ensure memory safety and robustness. For now, this
is a hobby language that I wrote to learn more about how compilers work. I may
iterate upon this to make it more robust in the future. I followed [this tutorial](https://www.youtube.com/playlist?list=PLI1h1vRqlHLNZAa2BEM9uZ2GEvUNYDasO)
to get everything set up initially, so check it out if you also want to create
your own compiler!

## 📕 Documentation

The documentation for the latest version of the terebinth compiler can be
found [here](https://docs.rs/terebinth/). All previously published versions
can be found on [crates.io](https://crates.io/crates/terebinth/versions),
and each version's respective documentation is accessible from there as well.

## 🔰 Getting Started

### Installation

The Terebinth compiler can be installed using cargo:

```shell
cargo install terebinth
```

or it can be built and installed from source:

```shell
git clone git@github.com:crhowell3/terebinth.git
cd terebinth
cargo install --path .
```

### Compiling Terebinth source code

Terebinth source files are suffixed with `.ter`. The compiler will check that
source files provided to it have this extension; even if a file contains valid
Terebinth syntax, if it is not correctly suffixed, it will be rejected by the
compiler.

Here is an example of a simple Terebinth program:

```shell
// main.ter
func add(x: int, y: int) -> int {
  return x + y
}

func main() {
  let z = add(3, 5)
}
```

To compile, simply run `terebinth main.ter`.

As of `version 0.1.0-alpha.1`, no assembling or linking occurs, so a binary is not
generated. The compiler will tokenize, parse, and construct an AST that is then
evaluated. With the code above, you should see this output:

```shell
> terebinth main.ter
func add(x: int, y: int) -> int {
  return x + y
}

func main() {
  let z = add(3, 5)
}


Result: Some(8) <<<< Result here

```

<p align="center">
  Copyright &copy; 2024-present
  <a href="https://github.com/crhowell3" target="_blank">Cameron Howell</a>
</p>
<p align="center">
  <a href="https://github.com/crhowell3/terebinth/blob/main/LICENSE"
    ><img
      src="https://img.shields.io/static/v1.svg?style=for-the-badge&label=License&message=MIT&logoColor=d9e0ee&colorA=282a36&colorB=b16286"
      alt="MIT License"
  /></a>
</p>
