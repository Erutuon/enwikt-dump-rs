#! /usr/bin/env bash

# Script to maintain a folder of dump files from a wiki,
# with the latest file easily locatable.
#
# Given a filename, downloads it to a filename with the date but without the wiki name,
# decompresses it, and links the filenames without the wiki name or the date
# to the latest filenames with the date.
# For example, $SCRIPT_NAME page will download
# <wiki>-<date>-page.sql.gz to <date>-page.sql.gz,
# decompress it to <date>-page.sql, and link page.sql to <date>-page.sql.
#
# Eases downloading and decompressing the latest dump file,
# or letting you know that the dump file has not been published yet.
# It takes one argument: the filename without the date,
# and with or without filename extensions.
# It will either download or indicate failure.
# It does not use the "latest" folder of the dump websites,
# but calculates the latest date.
# If you load completion.sh, you can use tab completion to help you
# remember names of the dump files.

# for example, http://dumps.wikimedia.your.org/enwiktionary/20190701/enwiktionary-20190701-all-titles.gz
PROTOCOL=https://
DOMAIN=dumps.wikimedia.your.org
WIKI=enwiktionary
YEAR=$(date +%Y)
MONTH=$(date +%m)
if [ "$(date +%d)" -ge 20 ]; then
	DAY=20;
else
	DAY=01;
fi
DATE=$YEAR$MONTH$DAY

case $1 in
	abstract | abstract.xml | abstract.xml.gz)
		FILE=abstract.xml.gz;;
	all-titles | all-titles.gz)
		FILE=all-titles.gz;;
	all-titles-in-ns0 | all-titles-in-ns0.gz)
		FILE=all-titles-in-ns0.gz;;
	babel | babel.sql | babel.sql.gz)
		FILE=babel.sql.gz;;
	category | category.sql | category.sql.gz)
		FILE=category.sql.gz;;
	categorylinks | categorylinks.sql | categorylinks.sql.gz)
		FILE=categorylinks.sql.gz;;
	change_tag | change_tag.sql | change_tag.sql.gz)
		FILE=change_tag.sql.gz;;
	change_tag_def | change_tag_def.sql | change_tag_def.sql.gz)
		FILE=change_tag_def.sql.gz;;
	externallinks | externallinks.sql | externallinks.sql.gz)
		FILE=externallinks.sql.gz;;
	geo_tags | geo_tags.sql | geo_tags.sql.gz)
		FILE=geo_tags.sql.gz;;
	image | image.sql | image.sql.gz)
		FILE=image.sql.gz;;
	imagelinks | imagelinks.sql | imagelinks.sql.gz)
		FILE=imagelinks.sql.gz;;
	iwlinks | iwlinks.sql | iwlinks.sql.gz)
		FILE=iwlinks.sql.gz;;
	langlinks | langlinks.sql | langlinks.sql.gz)
		FILE=langlinks.sql.gz;;
	page | page.sql | page.sql.gz)
		FILE=page.sql.gz;;
	page_props | page_props.sql | page_props.sql.gz)
		FILE=page_props.sql.gz;;
	page_restrictions | page_restrictions.sql | page_restrictions.sql.gz)
		FILE=page_restrictions.sql.gz;;
	pagelinks | pagelinks.sql | pagelinks.sql.gz)
		FILE=pagelinks.sql.gz;;
	pages-articles | pages-articles.xml | pages-articles.xml.bz2)
		FILE=pages-articles.xml.bz2;;
	pages-articles-multistream | pages-articles-multistream.xml | pages-articles-multistream.xml.bz2)
		FILE=pages-articles-multistream.xml.bz2;;
	pages-articles-multistream-index | pages-articles-multistream-index.txt | pages-articles-multistream-index.txt.bz2)
		FILE=pages-articles-multistream-index.txt.bz2;;
	pages-logging | pages-logging.xml | pages-logging.xml.gz)
		FILE=pages-logging.xml.gz;;
	pages-meta-current | pages-meta-current.xml | pages-meta-current.xml.bz2)
		FILE=pages-meta-current.xml.bz2;;
	pages-meta-history | pages-meta-history.xml | pages-meta-history.xml.bz2)
		FILE=pages-meta-history.xml.bz2;;
	protected_titles | protected_titles.sql | protected_titles.sql.gz)
		FILE=protected_titles.sql.gz;;
	redirect | redirect.sql | redirect.sql.gz)
		FILE=redirect.sql.gz;;
	site_stats | site_stats.sql | site_stats.sql.gz)
		FILE=site_stats.sql.gz;;
	siteinfo-namespaces | siteinfo-namespaces.json | siteinfo-namespaces.json.gz)
		FILE=siteinfo-namespaces.json.gz;;
	sites | sites.sql | sites.sql.gz)
		FILE=sites.sql.gz;;
	stub-articles | stub-articles.xml | stub-articles.xml.gz)
		FILE=stub-articles.xml.gz;;
	stub-meta-current | stub-meta-current.xml | stub-meta-current.xml.gz)
		FILE=stub-meta-current.xml.gz;;
	stub-meta-history | stub-meta-history.xml | stub-meta-history.xml.gz)
		FILE=stub-meta-history.xml.gz;;
	templatelinks | templatelinks.sql | templatelinks.sql.gz)
		FILE=templatelinks.sql.gz;;
	user_former_groups | user_former_groups.sql | user_former_groups.sql.gz)
		FILE=user_former_groups.sql.gz;;
	user_groups | user_groups.sql | user_groups.sql.gz)
		FILE=user_groups.sql.gz;;
	wbc_entity_usage | wbc_entity_usage.sql | wbc_entity_usage.sql.gz)
		FILE=wbc_entity_usage.sql.gz;;
	"")
		echo "Supply the name of a dump file to download.";
		exit -1;;
	*)
		echo "No subroutine programmed for file $1."
		exit -1;;
esac

         FULL_FILENAME="$WIKI-$DATE-$FILE"  #      download <wiki>-20191201-page.sql.gz
        LOCAL_FILENAME="$DATE-$FILE"        #       save as        20191201-page.sql.gz
             LINK_NAME="$FILE"              #     make link                 page.sql.gz
          DECOMPRESSED=${LOCAL_FILENAME%.*} # decompress to        20191201-page.sql
DECOMPRESSED_LINK_NAME=${FILE%.*}           #     make link                 page.sql

BACKSPACE=$'\r\e[0K'
if [[ -f "$LOCAL_FILENAME" && -s "$LOCAL_FILENAME" ]]; then
	echo "$LOCAL_FILENAME has already been downloaded."
else
	echo -n "Downloading $FULL_FILENAME from $DOMAIN..."
	if ! wget -q -O "$LOCAL_FILENAME" "$PROTOCOL$DOMAIN/$WIKI/$DATE/$FULL_FILENAME"; then
		echo "${BACKSPACE}Failed to download $FULL_FILENAME."
		rm "$LOCAL_FILENAME" # remove empty file
	else
		echo -n "${BACKSPACE}Downloaded $FULL_FILENAME from $DOMAIN "
		
		if ln -sf "$LOCAL_FILENAME" "$LINK_NAME"; then
			echo -n "and linked "
		else
            echo -n "but failed to link "
		fi
		echo "$LINK_NAME to it."
		
        # decompression filename extension
        case ${FILE##*.} in
            gz)
                DECOMPRESSOR=gunzip;;
            bz2)
                DECOMPRESSOR=bunzip2;;
            *)
                exit -1;;
		esac
		
		echo -n "Decompressing..."
		"$DECOMPRESSOR" -fk "$LOCAL_FILENAME"
        echo -n "${BACKSPACE}Decompressed $DECOMPRESSED "
		
		if ln -sf "$DECOMPRESSED" "$DECOMPRESSED_LINK_NAME"; then
            echo -n "and linked "
        else
            echo -n "but failed to link "
        fi
		echo "$DECOMPRESSED_LINK_NAME to it."
	fi
fi
