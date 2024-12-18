# rule replace

*replace lines in files with set of rules*

## Why?

### Indentation
Because python, yaml and similar are based on the proper identation. Using
there jinja/sed/awk - you name it - can cause a lot of errors. This is a
solution.

### Multiple replacement rules
You want multiple replacement rules? Are you tired of the chains of seds? Here
you go!

## Building

1. [Install rust](https://rustup.rs/)
2. Build the software

```bash
git clone https://github.com/Dzordzu/rule-replace
cd ./rule-replace
cargo build --release
```

## Usage example

In shell `rule-replace -c config.toml -f input.txt`

config.toml

```toml
[[rule]]
replace.lines_between = ["# BEGIN", "# END"]
keep_spaces = true

    [rule.with]
    template = "./template.j2"
    values = "./values.json"
```

For more see example directory. Inputs and expected outputs are presented
within each suite `tests` subdirectory.
