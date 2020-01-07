#! /usr/bin/env lua53

collectgarbage("setpause", 120)

local function print_redirects_in_namespaces(namespaces)
	local redirect_id_to_title = {}
	local namespaces_to_find = {}
	for _, namespace in ipairs(namespaces) do
		namespaces_to_find[namespace] = true
	end
	for page in require "iter_sql" "page" do
		if page.is_redirect and namespaces_to_find[page.namespace] then
			redirect_id_to_title[page.id] = page.title
		end
	end

	for redirect in require "iter_sql" "redirect" do
		local from_id = redirect.from
		local from_title = redirect_id_to_title[from_id]
		if from_title then
			print(from_title, redirect.title)
			redirect_id_to_title[from_id] = nil
		end
	end
	
	assert(
		not next(redirect_id_to_title),
		"a redirect page in page.sql was not found in redirect.sql")
end


local MAIN = 0
local APPENDIX = 100
local RECONSTRUCTION = 118
print_redirects_in_namespaces({ MAIN, APPENDIX, RECONSTRUCTION })