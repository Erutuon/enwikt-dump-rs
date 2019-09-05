# Makes pretty-printed JSON from all-headers subcommand more compact.
sd '("counts":)\s*\[\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)\s*\]' '$1 [$2,$3,$4,$5,$6,$7]' $1