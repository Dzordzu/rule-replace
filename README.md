# rule replace

*replace lines in files with set of rules*

## Building

```bash
git clone https://github.com/Dzordzu/rule-replace
cd ./rule-replace
cargo build --release
```

## Example

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

For more see example directory
