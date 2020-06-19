# Change this to the name of download.sh in your PATH.
set -l script_name download-dump

# Descriptions were generated in the following way as of the 2020-06-01 dump.
# Run this code in the browser JavaScript console on the dump page.
: '
console.log(
	JSON.stringify(
		Array.from(document.querySelector("body ul").children)
			.map(e => [
				e.getElementsByClassName("title")[0].innerText,
				Array.from(e.getElementsByClassName("file"))
					.map(e => e.innerText.match(/^[a-z-]+?-\d+-([^\.]+)/)[1])
			])
	)
)
'
# Copy the resulting JSON into this Lua script at "insert json here".
# The CJSON library is required.
# Change `download-dump` to the name of `download.sh` in your PATH.
# Run the script and paste the output below the first `complete` command here.
: '
local json_decode = require "cjson".decode
local completions_json = [=[insert JSON here]=]
local completions = json_decode(completions_json)
table.sort(
	completions,
	function(a, b)
		local a_, b_ = a[2][1], b[2][1]
		if a_ and b_ then
			return a_ < b_
		else
			return not a_
		end
	end)
for _, x in ipairs(completions) do
	local desc, names = table.unpack(x)
	if #names > 0 then
		io.write(
			[[complete -c download-dump -a "]],
			table.concat(names, " "),
			[[" -d "]],
			desc,
			[["\n]])
	end
end
'

complete -c $script_name -f -a '
all-titles change_tag_def abstract change_tag sites pages-articles stub-meta-current
stub-articles stub-meta-history all-titles-in-ns0 page pages-logging site_stats imagelinks iwlinks
category pages-meta-current langlinks page_restrictions geo_tags externallinks babel templatelinks
pagelinks page_props user_former_groups siteinfo-namespaces image pages-articles-multistream
pages-articles-multistream-index protected_titles redirect categorylinks wbc_entity_usage user_groups
pages-meta-history'

complete -c $script_name -a "abstract" -d "Extracted page abstracts for Yahoo"
complete -c $script_name -a "all-titles" -d "List of all page titles"
complete -c $script_name -a "all-titles-in-ns0" -d "List of page titles in main namespace"
complete -c $script_name -a "babel" -d "Language proficiency information per user."
complete -c $script_name -a "category" -d "Category information."
complete -c $script_name -a "categorylinks" -d "Wiki category membership link records."
complete -c $script_name -a "change_tag" -d "List of annotations (tags) for revisions and log entries"
complete -c $script_name -a "change_tag_def" -d "Annotation (tag) names and ids."
complete -c $script_name -a "externallinks" -d "Wiki external URL link records."
complete -c $script_name -a "geo_tags" -d "List of pages' geographical coordinates"
complete -c $script_name -a "image" -d "Metadata on current versions of uploaded media/files."
complete -c $script_name -a "imagelinks" -d "Wiki media/files usage records."
complete -c $script_name -a "iwlinks" -d "Interwiki link tracking records"
complete -c $script_name -a "langlinks" -d "Wiki interlanguage link records."
complete -c $script_name -a "page" -d "Base per-page data (id, title, old restrictions, etc)."
complete -c $script_name -a "page_props" -d "Name/value pairs for pages."
complete -c $script_name -a "page_restrictions" -d "Newer per-page restrictions table."
complete -c $script_name -a "pagelinks" -d "Wiki page-to-page link records."
complete -c $script_name -a "pages-articles" -d "Articles, templates, media/file descriptions, and primary meta-pages."
complete -c $script_name -a "pages-articles-multistream pages-articles-multistream-index" -d "Articles, templates, media/file descriptions, and primary meta-pages, in multiple bz2 streams, 100 pages per stream"
complete -c $script_name -a "pages-logging" -d "Log events to all pages and users."
complete -c $script_name -a "pages-meta-current" -d "All pages, current versions only."
complete -c $script_name -a "pages-meta-history" -d "All pages with complete page edit history (.bz2)"
complete -c $script_name -a "protected_titles" -d "Nonexistent pages that have been protected."
complete -c $script_name -a "redirect" -d "Redirect list"
complete -c $script_name -a "site_stats" -d "A few statistics such as the page count."
complete -c $script_name -a "siteinfo-namespaces" -d "Namespaces, namespace aliases, magic words."
complete -c $script_name -a "sites" -d "This contains the SiteMatrix information from meta.wikimedia.org provided as a table."
complete -c $script_name -a "stub-meta-history stub-meta-current stub-articles" -d "First-pass for page XML data dumps"
complete -c $script_name -a "templatelinks" -d "Wiki template inclusion link records."
complete -c $script_name -a "user_former_groups" -d "Past user group assignments."
complete -c $script_name -a "user_groups" -d "User group assignments."
complete -c $script_name -a "wbc_entity_usage" -d "Tracks which pages use which Wikidata items or properties and what aspect (e.g. item label) is used."
