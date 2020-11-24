#! /usr/bin/env fish

set -l input_dump_date $DUMP_DATE
set -l dump_date
if test -z $input_dump_date
	set -l year (date +%Y)
	set -l month (date +%m)
	set -l day
	if test (date +%d) -ge 20;
		set day 20
	else
		set day 01
	end
	set dump_date $year$month$day
else
	set dump_date $input_dump_date
end
echo Working with $dump_date dump

if not begin; set -q CBOR_DIR; and set -q DUMP_DIR; end
	echo 'both $CBOR_DIR and $DUMP_DIR required'
	exit 1
end
set -l dump_prefix $DUMP_DIR/$dump_date-

set -l template_redirects_bin template_redirects
set -l template_redirects_json template_redirects.json
if which $template_redirects_bin > /dev/null
	echo "Generating $template_redirects_json"
	$template_redirects_bin {$dump_prefix}page.sql {$dump_prefix}redirect.sql > $template_redirects_json
else if not test -f $template_redirects_json
	echo "$template_redirects_bin binary (from parse-wiki-text) not found on \$PATH" \
		"and $template_redirects_json not found"
	exit 1
end

set -l template_names template_names.txt
set -l template_names_cbor (readlink -f template_names_cbor.txt)
cat $template_names | lua lua/add_template_redirects.lua "%s.cbor" $template_redirects_json \
	> $template_names_cbor

set -l orig_dir $PWD
set -l dated_cbor_dir $CBOR_DIR/$dump_date
if not begin; mkdir -p $dated_cbor_dir; and cd $dated_cbor_dir; end;
	echo "Failed to create output directory"; exit 1
end

echo "Dumping parsed templates in $dated_cbor_dir"
wiktionary-data dump-parsed-templates \
	--input $DUMP_DIR/$dump_date-pages-articles.xml \
	--templates $template_names_cbor \
	--namespaces main,reconstruction,appendix \
	--format cbor \
	--include-text

cd $orig_dir
