#! /usr/bin/env fish

# pages-articles.xml must be symlinked to filename containing date (yyyymmdd).
set -l date (readlink -f pages-articles.xml | \
	rg --only-matching '(?P<y>\d{4})(?P<m>\d{2})(?P<d>\d{2})' --replace '$y-$m-$d')

if test -z "$date"
	echo "Could not determine date of pages-articles.xml; will not generate header statistics or filter headers."
	exit
end

set -l all_headers all_headers/"$date".json
if test ! \( -f "$all_headers" -a -s "$all_headers" \)
	echo 'generating header statistics'
	wiktionary-data all-headers \
		--namespaces main \
		--pretty \
		| sd '("counts":)\s*\[\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)\s*\]' \
			'$1 [$2,$3,$4,$5,$6,$7]' \
			> "$all_headers"
	ln -sf "$date".json all_headers/latest.json
end

set -l filtered_headers filtered_headers/"$date".json
if test ! \( -f "$filtered_headers" -a -s "$filtered_headers" \)
	set -l language_names language_names.txt
	echo 'getting data on language names'
	process-with-lua text \
		-i pages-articles.xml \
		-n module \
		-e 'if page.title == "Module:languages/canonical names" then
			print(page.text)
			return false
		end
		return true' \
		> language_name_to_code.lua;
		or begin;
			echo "Error while finding Module:languages/canonical names;"
			exit -1;
		end;
	lua -e 'for name in pairs(require "language_name_to_code") do print(name) end' > "$language_names"
	echo 'filtering headers'
	wiktionary-data filter-headers \
		--namespaces main,reconstruction \
		--top-level-headers "$language_names" \
		--other-headers "correct_headers.txt" \
		--pretty \
		> "$filtered_headers"
	ln -sf "$date".json filtered_headers/latest.json
end

set -l summary "update from $date dump"
env PYTHONPATH="$HOME/pywikibot" ./save_json.py \
	"<pre><nowiki>\n{}</nowiki></pre>" \
	"User:Erutuon/mainspace headers/data" \
	"$summary" \
	"$all_headers"
env PYTHONPATH="$HOME/pywikibot" ./update_data_page.py \
	"User:Erutuon/mainspace headers" \
	"$date" \
	"$summary"
env PYTHONPATH="$HOME/pywikibot" ./update_data_page.py \
	"User:Erutuon/language headers" \
	"$date" \
	"$summary"
	
env PYTHONPATH="$HOME/pywikibot" ./save_json.py \
	"{}" \
	"User:Erutuon/mainspace headers/possibly incorrect/json" \
	"$summary" \
	"$filtered_headers"
env PYTHONPATH="$HOME/pywikibot" ./update_data_page.py \
	"User:Erutuon/mainspace headers/possibly incorrect" \
	"$date" \
	"$summary"
env PYTHONPATH="$HOME/pywikibot" ./update_data_page.py \
	"User:Erutuon/abbreviation headers" \
	"$date" \
	"$summary"
env PYTHONPATH="$HOME/pywikibot" ./update_data_page.py \
	"User:Erutuon/numbered part of speech headers" \
	"$date" \
	"$summary"
