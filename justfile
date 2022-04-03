# ~~~ Constants ~~~
LibName                 := "yary"
LibMsrv                 := "1.53"

# ~~~ Environment overrides ~~~
Color                   := env_var_or_default("YARY_COLOR", env_var_or_default("CARGO_TERM_COLOR", "1"))
Dryrun                  := env_var_or_default("YARY_DRYRUN", None)
Features                := env_var_or_default("YARY_FEATURES", None)
Profile                 := env_var_or_default("YARY_PROFILE", None)
RustFlags               := env_var_or_default("YARY_RUSTFLAGS", env_var_or_default("RUSTFLAGS", None))
DocFlags                := env_var_or_default("YARY_RUSTDOCFLAGS", env_var_or_default("RUSTDOCFLAGS", None))

# Default to listing recipes
_default:
  @just --list --list-prefix '  > '

# Display the recipe's steps
show recipe:
  @just --show {{recipe}}

# Open library documentation in your browser
docs: (_build-docs "open")

# Check the library for syntax errors
check:
  @$Say "Checking library for syntax errors..."
  @$Cargo check

# Print entire changelog
changelog range=None: (_changelog range)

# Print patch notes for unreleased changes
patchnotes range=None: (_changelog if range != None { range } else { LibVersion + ".." } "all")

# Build the library
build: (_build Profile Features)

# Build library documentation
build-docs: _build-docs

# Display dependency tree of the library
deps edges="normal": _need_tree
  @$Cargo tree --edges {{edges}}

# Display reverse dependency tree of the given crate
rdeps crate=LibName edges="normal": _need_tree
  @$Cargo tree --invert --package {{crate}} --edges {{edges}}

# Check for unused dependencies
udeps: _need_udeps
  @$Say "Checking for unused dependencies..."
  @$Cargo +nightly udeps

# Run unit tests
test selector=None: (_test "lib" Profile Features selector)

# Run documentation tests
test-docs: (_test "doc" Profile Features)

# Run example tests
test-examples: (_test "examples" Profile Features)

# Run entire test suite
test-all: test test-docs test-examples lint-docs

# Clean the local build artifacts
clean:
  @$Say "Cleaning build artifacts..."
  @$Cargo clean

# Clean the documentation artifacts
clean-docs:
  @$Say "Cleaning documentation artifacts..."
  @$Cargo clean --doc

# Clean the cargo binaries used by this repo (modifies ~/.cargo/bin)
clean-bins: && _clean_deps
  @$Say "Cleaning Cargo binaries..."

# Clean Cargo's cache (modifies ~/.cargo)
clean-cache: _need_cache
  @$Say "Cleaning Cargo cache..."
  @$Cargo cache --auto-clean

# Remove all local artifacts
clean-all: clean clean-docs

# Format library code
fmt: (_format)

# Initialize this library checkout, installing the required rustc version & components
fresh-system: (_fresh-system LibMsrv) install-bins check

# Install the cargo binaries used by this repo
install-bins update=None: (_build_deps update)

# Prune git local branches which previously had a tracking branch
git-branch-prune:
  @$Say "Pruning [gone] branches..."
  @$Git fetch --prune --all --quiet && \
    git for-each-ref --format '%(refname) %(upstream:track)' refs/heads \
    | awk '$2 == "[gone]" {sub("refs/heads/", "", $1); print $1}' \
    | xargs -n1 ${NODRYRUN:+$Git branch -D}

# Run comprehensive check suite
git-pre-push: lint build lint-docs udeps audit test-all
alias pp := git-pre-push

# Lint library code
lint: (_format "check") _clippy

# Lint library documentation
lint-docs: (_build-docs "no-open" "check")

# Audit dependencies, checking for any known vulnerabilities or CVEs
audit: _need_audit
  @$Say "Auditing dependencies..."
  @$Say "[{{C_RED}}SKIP{{C_RESET}}] Tool is seg-faulting. See {{C_YELLOW}}https://github.com/rustsec/rustsec/issues/466{{C_RESET}}"
  @#$Cargo audit

# Update the library's version to the specified
bump-version to: (_bump-cargo-version "Cargo.toml" to) check (_bump-git-version to)
  @$Say "Run the following command when ready"
  @echo "{{C_RED}}==> {{C_GREEN}}git push --atomic origin master v{{to}}{{C_RESET}}"

# ~~~ Private recipes ~~~

# Run cargo test with the given suite, profile, features and selector (if any)
_test $suite=None $profile=Profile $features=Features selector=None:
  @$Say "Running tests" \
    "${suite:+{{C_GREEN}}suite:{{C_YELLOW}}$suite{{C_RESET}}}" \
    "${features:+{{C_GREEN}}features:{{C_YELLOW}}$features{{C_RESET}}}" \
    {{ if selector != None { C_GREEN + "selector:" + C_YELLOW + selector + C_RESET } else { None } }} \
    | xargs
  @$Cargo test \
    ${suite:+--$suite} \
    ${features:+--features $features} \
    {{ if profile =~ '(?i)^release$' { "--release" } else { None } }} \
    {{selector}}

# Run rustfmt with nightly so it understands our .rustfmt.toml rules
_format $check=None:
  @$Say "Formating library..."
  @$Cargo +nightly fmt ${check:+--check}

# Run clippy with the correct args
_clippy:
  @$Say "Linting library..."
  @$Cargo clippy -- -D clippy::all -W clippy::style

# Build the library
_build $profile=Profile $features=Features:
  @$Say "Building library..."
  @$Cargo build \
    {{ if profile =~ '(?i)^release$' { "--release" } else { None } }} \
    ${features:+--features $features}

# Build our documentation
_build-docs open="no" check="no" features=Features:
  #!/bin/sh
  set -eu

  {{ if check =~ '(?i)^check|true|yes|1$' { 'export RUSTDOCFLAGS="$RUSTDOCFLAGS -Dwarnings"' } else { None } }}

  $Say "Building library docs..."
  $Cargo +nightly doc \
    --document-private-items \
    {{ if features != None { "--features " + features } else { "--all-features" } }} \
    {{ if open =~ '(?i)^open|true|yes|1$' { "--open" } else { None } }}

@_changelog range=None $strip=None $output=None +extra=None: _need_cliff
  $Cliff {{extra}} ${strip:+--strip $strip} ${output:+--output $output} {{range}}

# Tiny perl script to bump the Cargo version field
@_bump-cargo-version file $version temp=`mktemp`:
  $Say "Bumping {{file}} version to $version"
  $Perl -spe \
    {{ if Dryrun != None { '"' } else { '' } }} \
    'if (/^version/) { s/("[\w.]+")/"$version"/ }' \
    -- -version=$version < {{file}} > {{temp}} \
    && mv -f {{temp}} {{file}} \
    {{ if Dryrun != None { '"' } else { '' } }}

# Add commit + tag to Git for the provided version
@_bump-git-version version temp=`mktemp`: (_changelog LibVersion + ".." "all" temp "--tag" version "--body" '"$(cat .git-cliff/tag.tera)"') (_changelog None None "CHANGELOG.md" "--tag" version)
  $Say "Adding git tag v{{version}} to HEAD"
  if ! git branch --show-current | grep -qF 'master'; then \
    $Say 'Refusing to set git tag, branch is not master' && false; \
  fi
  $Git add Cargo.toml Cargo.lock CHANGELOG.md
  $Git commit -m 'chore: release v{{version}}'
  $Git tag -a v{{version}} -F {{temp}}

# Install rustc version that we use in this repo + all the components we're expecting
@_fresh-system msrv=RustVersion:
  $Say "Installing Rust version {{msrv}}..."
  $Rustup install {{msrv}}
  $Rustup install nightly
  $Say "Setting override for {{justfile_directory()}}"
  $Rustup override set {{msrv}}
  $Say "Installing rustc components..."
  $Rustup component add rustfmt clippy rust-src
  $Rustup component add --toolchain nightly rustfmt clippy rust-docs

# ~~~ Cargo binary management ~~~

_build_deps update=None: (_need_cache update) (_need_udeps update) (_need_audit update) (_need_tree update) (_need_cliff update)
_clean_deps: _clean_cache _clean_udeps _clean_audit _clean_tree _clean_cliff

# Cargo udeps
_need_udeps update=None: (_need "udeps" update None "nightly")
_clean_udeps: (_clean_need "udeps" "nightly")

# Cargo cache
_need_cache update=None: (_need "cache" update "no-default-features,ci-autoclean" "nightly")
_clean_cache: (_clean_need "cache")

# Cargo audit
_need_audit update=None: (_need "audit" update None "nightly")
_clean_audit: (_clean_need "audit")

# Cargo tree
_need_tree update=None: (_need "tree" update None "nightly")
_clean_tree: (_clean_need "tree")

# Git-cliff
_need_cliff update=None: (_need "git-cliff" update None "nightly" None)
_clean_cliff: (_clean_need "git-cliff")

# Specify a dependency on a cargo binary
@_need crate $update=None features=None $nightly=None prefix="cargo-":
  needed={{ if prefix != None { prefix + crate } else { crate } }}; \
    {{ if update == None { "command -v $needed 2>/dev/null 1>/dev/null" } else { "false" } }} \
    || $Cargo ${nightly:++nightly} install \
      ${update:+--force} \
      {{ if features =~ "no-default-features" { "--no-default-features" } else { None } }} \
      {{ if features != None { "--features " + replace(features, "no-default-features,", None)} else { None } }} \
      $needed

# Clean a cargo binary
@_clean_need crate $nightly=None:
  needed=cargo-{{crate}}; \
    command -v $needed 2>/dev/null 1>/dev/null \
    && $Cargo ${nightly:++nightly} uninstall $needed \
    || true

# ~~~ Global shell variables ~~~
export Say              := "echo " + C_RED + "==> " + C_RESET + BuildId
export Cargo            := if Dryrun == None { "cargo" } else { DryrunPrefix + "cargo" }
export Rustup           := if Dryrun == None { "rustup" } else { DryrunPrefix + "cargo" }
export Git              := if Dryrun == None { "git" } else { DryrunPrefix + "git" }
export Sed              := if Dryrun == None { "sed" } else { DryrunPrefix + "sed" }
export Perl             := if Dryrun == None { "perl" } else { DryrunPrefix + "perl" }
export Cliff            := if Dryrun == None { "git-cliff" } else { DryrunPrefix + "git-cliff" }
export DRYRUN           := if Dryrun == None { None } else { "1" }
export NODRYRUN         := if Dryrun == None { "1" } else { None }
export RUSTFLAGS        := RustFlags
export RUSTDOCFLAGS     := DocFlags

# Nicer name for empty strings
None                    := ""

# ~~~ Contextual information ~~~
GitCommitish            := if `git tag --points-at HEAD` != None {
                              `git tag --points-at HEAD`
                           } else if `git branch --show-current` != None {
                              `git branch --show-current`
                           } else {
                              `git rev-parse --short HEAD`
                           }
RustVersion             := `rustc --version | cut -d' ' -f2`
LibVersion              := `git describe --abbrev=0 2>/dev/null || echo -n "0.0.0"`

DryrunPrefix            := "echo " + "[" + C_GREEN + "DRYRUN" + C_RESET + "] "
BuildId                 := "[" + C_YELLOW + RustVersion + C_RESET + "/" + C_GREEN + LibName + C_RESET + "@" + C_CYAN + GitCommitish + C_RESET + "]"

# ~~~ Color Codes ~~~
C_ENABLED   := if Color =~ '(?i)^auto|always|yes|1$' { "1" } else { None }

C_RESET     := if C_ENABLED == "1" { `echo -e "\033[0m"`    } else { None }
C_BLACK     := if C_ENABLED == "1" { `echo -e "\033[0;30m"` } else { None }
C_RED       := if C_ENABLED == "1" { `echo -e "\033[0;31m"` } else { None }
C_GREEN     := if C_ENABLED == "1" { `echo -e "\033[0;32m"` } else { None }
C_YELLOW    := if C_ENABLED == "1" { `echo -e "\033[0;33m"` } else { None }
C_BLUE      := if C_ENABLED == "1" { `echo -e "\033[0;34m"` } else { None }
C_MAGENTA   := if C_ENABLED == "1" { `echo -e "\033[0;35m"` } else { None }
C_CYAN      := if C_ENABLED == "1" { `echo -e "\033[0;36m"` } else { None }
C_WHITE     := if C_ENABLED == "1" { `echo -e "\033[0;37m"` } else { None }
