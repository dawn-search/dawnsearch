# Data

Data is stored in dawnsearch.sql and usearch.index. The default directory is ./data, but you can change this through DawnSearch.toml.

If you rsync them, it's useful to use --compress and --progess.

rsync --progress --compress dawnsearch/store/* server:path
