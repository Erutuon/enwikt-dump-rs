#! /usr/bin/env fish

set -l template_names template_names.txt
set -l template_names_cbor template_names_cbor.txt
if test ! \( -f "$template_names_cbor" -a -s "$template_names_cbor" \)
	cat $template_names | lua lua/add_template_redirects.lua > $template_names_cbor
	
	cd cbor
	echo "dumping parsed templates"
	wiktionary-data dump-parsed-templates --input ../pages-articles.xml \
		--templates ../$template_names_cbor \
		--namespaces main,reconstruction,appendix \
		--format cbor \
		--include-text
	cd ..
end