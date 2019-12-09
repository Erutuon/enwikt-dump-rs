#! /usr/bin/env fish

set -l template_names template_names.txt
set -l template_names_cbor template_names_cbor.txt
if test ! \( -f "$template_names_cbor" -a -s "$template_names_cbor" \)
	echo "adding template redirects to $template_names"
	wiktionary_dump2 add-template-redirects --suffix .cbor "$template_names"
	mv "$template_names".new "$template_names_cbor"
	
	cd cbor
	echo "dumping parsed templates"
	wiktionary_dump2 dump-parsed-templates --input ../pages-articles.xml \
		--format cbor --templates ../"$template_names_cbor" \
		--namespaces main,reconstruction,appendix
	cd ..
end