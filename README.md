# focus

A DNS override that results in the blocking of sites.

Useful for those struggling with self control on X or other.

### installing

Install system wide as `focus`.

```bash
cargo install --path .
```

### usage

run `focus` once installed and it will show you usage

`focus on` reroutes the DNS of sites in the focus file.
`focus off` drops the rerouting.
`focus edit` will open a file to edit in vim.
There are examples in that file of how to block a site.
Add, delete, comment as you wish.
In Vim, `[escape] + :x + [enter]` to save and exit.

