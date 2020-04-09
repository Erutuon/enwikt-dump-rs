#! /usr/bin/env bash

# Change this to the name of download.sh in your PATH.
SCRIPT_NAME=download-dump

complete -W 'abstract all-titles all-titles-in-ns0 babel category categorylinks
change_tag change_tag_def externallinks geo_tags image imagelinks iwlinks
langlinks page page_props page_restrictions pagelinks pages-articles
pages-articles-multistream pages-articles-multistream-index pages-logging
pages-meta-current pages-meta-history protected_titles redirect site_stats
siteinfo-namespaces sites stub-articles stub-meta-current stub-meta-history
templatelinks user_former_groups user_groups wbc_entity_usage' "$SCRIPT_NAME"