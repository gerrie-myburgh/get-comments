echo usage folder-name file-name destination-adoc-file
cd "$1"
fswatch "$2" --event=Updated | while read file; do
    filename=$(basename "$file")
    echo "$filename"
    asciidoctor "$filename" -o "$3" 2>/dev/null
done
