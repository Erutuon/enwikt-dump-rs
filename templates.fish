#! /usr/bin/env fish

set -l template_names template_names.txt
set -l template_names_cbor template_names_cbor.txt
cat $template_names | lua lua/add_template_redirects.lua "%s.cbor" template_redirects.json \
	> $template_names_cbor

if not begin; mkdir -p cbor; and cd cbor; end;
	echo "Failed to create output directory"; exit -1
end
echo "dumping parsed templates"
wiktionary-data dump-parsed-templates \
	--input ../pages-articles.xml \
	--templates ../$template_names_cbor \
	--namespaces main,reconstruction,appendix \
	--format cbor \
	--include-text
cd ..