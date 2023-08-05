Data is stored in dawnsearch.sql and usearch.index.

If you rsync them, it's useful to use --compress and --progess.

rsync --progress --compress dawnsearch/store/* server:path
