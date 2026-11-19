<p align="center">
  <img src="https://diegopn.github.io/potoo-site/src/assets/images/language-interpreter-icon.svg?v=1c651ca7" alt="Potoo IDE language interpreter icon" width="120" />
</p>

<h1 align="center">Jeem Interpreter</h1>

<p align="center">A Rust interpreter for source-documented features of historical Arabic Jeem.</p>

<p align="center">
  <a href="https://github.com/diegopn/potoo-jeem-interpreter/releases">Download</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/jeem/">Website</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/jeem/">User manual</a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://doc.rust-lang.org/cargo/"><img alt="Cargo" src="https://img.shields.io/badge/Cargo-DEA584?style=for-the-badge&logo=rust&logoColor=white"></a>
  <img alt="Rust library" src="https://img.shields.io/badge/Rust_library-4A86CF?style=for-the-badge">
  <img alt="CLI" src="https://img.shields.io/badge/CLI-6A4C93?style=for-the-badge">
</p>

This Rust package is part of the Potoo IDE ecosystem and is distributed independently from the desktop interface. Integration with the IDE has not been implemented or verified. It implements features recovered from the 2007 technical study and a 15-lesson course; it does not claim compatibility with a current Jeem release or recreate the original Al-Khwarizmi environment. See [REFERENCE-COVERAGE.md](REFERENCE-COVERAGE.md) for the feature matrix and unresolved source questions.

## Implemented language forms

- Arabic declarations for integer, small integer, real, logical, character, pointer, and file variables. Fixed-size arrays support scalar and record elements, one or more dimensions, and sizes computed from earlier integer constants. Named type aliases and compound records accept the documented `مركب`, `سجل`, `كيان`, and `وحدة` markers.
- Numeric, logical, character, and text constants, including compile-time expressions over literals and previously declared constants. Assignment works with `أجعل` or with the variable alone.
- Arithmetic `+`, `−`, `×`, `÷`, and integer quotient; comparisons include `=؟`, `<>`, `<`, `≤`, `>`, and `≥`; logical operators include `و`, `أو`, and `نفي`. Parenthesized expressions and absolute value `||…||` are supported.
- `أقرأ` for scalar numeric and character values, comma-separated targets, and one-dimensional character arrays as text. `أكتب` supports numeric/character values, text literals, and `سطر`; logical and compound values cannot be written directly.
- `إذا`/`وإلا`, counted loops (`أكرر … مرة`), while loops (`أكرر طالما`), repeat-until loops (`أكرر { … }حتى …`), and inclusive range loops (`أكرر من … إلى …`, with optional `بخطوة`). `أنتهي` stops execution.
- Algorithms and functions with typed scalar and array parameters, local variable and constant declarations, scalar return values, recursion, and address-passed mutable lvalues of matching types. Rvalue arguments use temporary parameter cells. A `الثابت` parameter is copied so changes inside the routine do not alter the caller's value; the routine may still change its local parameter.
- Array indexing in expressions, assignments, input targets, and routine arguments, including indexed record fields such as `palette[index].red`. Arrays passed to routines share storage unless the formal is marked `الثابت`; character text literals become temporary one-dimensional character arrays. The source example uses index `1` for its first character and `0` as a text terminator; this implementation follows that convention for character arrays.
- Record declarations, field initialization through assignments, nested records, dotted field reads and writes, and copy assignment between values of the same named type. Primitive fields begin uninitialized.
- File open, read, write, and close statements using the lesson's documented Arabic forms. The `Host` implementation owns file paths and handles; the command-line host has no file access.
- Point, chained line, circle, ellipse, rectangle, and triangle commands; persistent stroke and fill colors, filled shapes, canvas fill, and positioned or cursor-relative labels through the typed `Host::draw_command` service. Screen-bound constants `حدس` and `حدع` use `Host::drawing_bounds`. Circles also remain compatible with the earlier `Host::draw_circle` callback. RGB channels are constrained to the documented 0–100 range.
- The `أدرج` and `بدل` compiler directives, including escaped quotes, macro substitution, recursive `.jeem` file inclusion, and library lookup beside the source file and under `common libs` beside the CLI executable. The five recovered standard libraries are implemented: math constants and functions; the `كلمة` type plus `أبجدي`, `طول_الكلمة`, `أنسخ_كلمة`, `أقارن_كلمتين`, and `عكس`; color aliases and palette routines; complex-number operations; and synonym substitutions. `نسخ_كلمة` is accepted as a compatibility alias for the documented `أنسخ_كلمة` routine.
- Built-in `جذر`, `صحيح`, `جب` (sine), `تجب` (cosine), and `لغ` (natural logarithm) calls are available; `رياضيات` adds `ظل`, `تظل`, `باقي`, and `تدوير`. The trigonometric functions use radians, as in the course's conversion example.
- Full-line `!!` comments, multiline `*( … )*` comments, source-line diagnostics, checked integer arithmetic, and a configurable execution-step limit.

## Source-defined gaps and behavior limits

The available sources name more language facilities than they specify. The parser rejects or reports these gaps instead of silently treating them as compatible:

- **Records and fields:** initialization defaults, structural compatibility, and pointer/reference behavior for records are not specified. `check(&program)` validates field paths and includes record-field types in static expression checks; uninitialized field reads are still detected at runtime.
- **Pointers:** pointer variables are declared, but allocation, dereference, null values, and pointer arithmetic are not fully specified.
- **Files:** append mode, encoding, record boundaries, and cleanup after a runtime error are not specified. The CLI has no file host.
- **Graphics:** the course specifies the geometric command forms implemented by the parser, but the original drawing surface is not recreated. Coordinate orientation, precision, label-cursor behavior, and drawing persistence remain the host's responsibility where the lessons do not define them. The command-line host has no drawing service.
- **Libraries and directives:** the recovered math, text, color, complex-number, and synonym libraries are implemented or supplied by the host. The callable contracts for `عجب`, `عتجب`, `عظل`, and `جديد` remain unavailable. The CLI host has no file or drawing service, so file-backed palettes and graphics require an embedding host.
- **Integer widths and conversion:** the sources do not give the numeric range for `صغير`; it currently uses the same storage representation as `صحيح`. `صحيح(real)` truncates toward zero as an explicit profile policy.
- **Execution details:** the sources do not settle all operator precedence, variable initialization, real-number precision, or negative integer-quotient rounding rules. `÷` produces a real value; integer quotient truncates toward zero. Range ends are inclusive, and counted loops with nonpositive counts run zero times.
- **Text arrays:** array sizes are fixed at declaration. Character arrays use one-based indexing and a zero terminator based on the course example; this differs from many host-language array conventions.
- **Routine parameters:** the course permits some numeric conversions for writable actual variables. This profile aliases matching types and passes other compatible scalar values through a temporary cell; converted writable actuals are not copied back. The course exercises confirm arrays as routine arguments and describe text parameters as character arrays, but do not show a complete formal array declaration. This profile accepts `name: صفيفة من <scalar type>` and derives dimensions from the actual array. Rank and bounds are checked when the routine indexes the parameter.
- **IDE compatibility:** this crate is a direct interpreter with a CLI and host API. It does not provide the original editor, compiler, debugger, graphics window, or full operating-system integration.

## Library use

Call `parse(source)` for programs using the recovered standard libraries, or `parse_with_library_loader(source, loader)` when a caller supplies custom `.jeem` files. Then call `check(&program)` and run it with `Interpreter::new(host).run(&program)`. Implement `Host` to provide input and output, typed drawing commands, drawing bounds, and file operations. Drawing and file methods have default unavailable-service implementations. `Execution` returns assigned variables and the executed-step count.

## Command line

Save a minimal program in `example.jeem`:

```text
أكتب 42, سطر
```

Run it from this repository or after installing the command:

```sh
cargo run --locked -- example.jeem
potoo-jeem example.jeem
```

The CLI reads the selected UTF-8 source file and connects `أقرأ` and `أكتب` to standard input and output. Custom libraries are loaded as `<name>.jeem` from the source directory first, then from `common libs` beside the executable. It has no file or graphics service, so programs that use those capabilities need an embedding host.

### Build and install from source

Install Rust 1.93 or later, then build and install the CLI from this repository:

```sh
cargo build --release --locked --bin potoo-jeem
cargo install --path . --locked --bin potoo-jeem
```

Cargo installs the command in its configured binary directory, usually `~/.cargo/bin`. Ensure that directory is on `PATH`, then run `potoo-jeem example.jeem`.

### Install a prebuilt release

After a release is published, download `SHA256SUMS` and the archive for your platform from the [GitHub Releases page](https://github.com/diegopn/potoo-jeem-interpreter/releases). The archive names are:

- Linux x86_64 GNU: `potoo-jeem-interpreter-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- Windows x86_64 MSVC: `potoo-jeem-interpreter-<tag>-x86_64-pc-windows-msvc.zip`
- Intel macOS: `potoo-jeem-interpreter-<tag>-x86_64-apple-darwin.tar.gz`
- Apple silicon macOS: `potoo-jeem-interpreter-<tag>-aarch64-apple-darwin.tar.gz`

Replace `<tag>` with the downloaded release tag and use the filename for your platform. The Linux build is made on Ubuntu 22.04 and may require a sufficiently recent glibc. On Linux or macOS, set `archive` to the exact downloaded filename and verify its matching checksum entry before extracting:

```sh
set -euo pipefail
archive="potoo-jeem-interpreter-<tag>-x86_64-unknown-linux-gnu.tar.gz"
matches="$(awk -v archive="$archive" '$2 == archive { count++; line = $0 } END { if (count != 1) exit 1; print line }' SHA256SUMS)"
if command -v sha256sum >/dev/null 2>&1; then
  printf '%s\n' "$matches" | sha256sum --check -
else
  printf '%s\n' "$matches" | shasum -a 256 --check -
fi
mkdir -p "$HOME/.local/bin"
tar -xzf "$archive"
install -m 0755 "${archive%.tar.gz}/potoo-jeem" "$HOME/.local/bin/potoo-jeem"
export PATH="$HOME/.local/bin:$PATH"
```

For Intel or Apple silicon macOS, use the corresponding archive name above. Add the `PATH` line to `~/.profile` or your shell configuration to keep it for future sessions. On Windows PowerShell, use the Windows archive and verify the exact entry before extraction:

```powershell
$archive = "potoo-jeem-interpreter-<tag>-x86_64-pc-windows-msvc.zip"
$checksumEntries = @(Get-Content .\SHA256SUMS | Where-Object { $_ -match "^\w+\s+$([regex]::Escape($archive))$" })
if ($checksumEntries.Count -ne 1) { throw "Expected one SHA256SUMS entry for $archive" }
$expected = ($checksumEntries[0] -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
if ($actual -ne $expected) { throw "SHA-256 mismatch for $archive" }
Expand-Archive -Path $archive -DestinationPath .
$bundleDir = [IO.Path]::GetFileNameWithoutExtension($archive)
$installDir = Join-Path $env:LOCALAPPDATA "Programs\potoo-jeem"
New-Item -ItemType Directory -Force $installDir | Out-Null
Copy-Item (Join-Path $bundleDir "potoo-jeem.exe") $installDir
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (($userPath -split ';') -notcontains $installDir) {
  $updatedPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $installDir } else { "$userPath;$installDir" }
  [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
}
$env:Path = "$installDir;$env:Path"
```

The Windows commands add the user-writable install directory to your user `PATH`; open a new terminal to use the command in future sessions.

## References

- [Taha Zerrouki, comparative study of Jeem, Zay, and Arabic Logo (2007)](https://arabproglang.wordpress.com/wp-content/uploads/2011/01/arabproglang.pdf), Jeem section, pp. 5–8. A historical technical snapshot, not a current normative specification.
- [Author-attributed 15-lesson Jeem course index](https://isnaha.com/index.php/مشاريع-التخرج/content/اللغة-ج). The course credits Dr. Muhammad Ammar al-Salka as original author and identifies the published material as edited.
- [Lesson 4: variables, constants, and assignment](https://isnaha.com/index.php?id=871%3A%D8%A7%D9%84%D8%AF%D8%B1%D8%B3-4-%D8%A7%D9%84%D9%85%D8%AA%D8%BA%D9%8A%D8%B1%D8%A7%D8%AA-%D9%88%D8%A7%D9%84%D8%AB%D9%88%D8%A7%D8%A8%D8%AA-%D9%88%D8%A7%D9%84%D8%A5%D8%B3%D9%86%D8%A7%D8%AF&option=com_k2&view=item).
- [Lesson 5: input and output](https://isnaha.com/index.php/مشاريع-التخرج/الدرس-5-قراءة-وكتابة-المعطيات).
- [Lesson 10: compound types](https://isnaha.com/index.php/مشاريع-التخرج/الدرس-10-البنيات).
- [Lesson 11: algorithms and functions](https://isnaha.com/index.php/مشاريع-التخرج/الدرس-11-الخوارزميات-والدوال).
- [Lesson 12: files](https://isnaha.com/index.php/مشاريع-التخرج/الدرس-12-الملفات).
- [Lesson 13: geometric graphics](https://isnaha.com/index.php/مشاريع-التخرج/الدرس-13-الرسومات-الهندسية).
- [Lesson 15: standard libraries](https://isnaha.com/index.php/مشاريع-التخرج/الدرس-15-المكتبات-الأساسية-للغة-ج).
- [Creator-attributed circle example](https://to-linux.blogspot.com/2015/09/blog-post_26.html), reproducing the circle-area example and reserved-word material.
- [Potoo Jeem manual](https://diegopn.github.io/potoo-site/manual/jeem/), which summarizes source evidence and its limits.

## Support

If this project is useful to you, you can support its continued development:

<p align="left">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## Project policies

- [Code of conduct](CODE_OF_CONDUCT.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)
