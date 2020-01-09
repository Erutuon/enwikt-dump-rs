#! /usr/bin/env lua53

collectgarbage("setpause", 120)

local function normalize_namespace_name(name)
	return (name
		:gsub(
			"^(.)(.+)$",
			function(first, rest)
				return first:upper() .. rest:lower()
			end)
		:gsub("_", " "))
end

local namespaces = require "namespaces"
local namespace_id_to_name = {}
local namespace_name_to_id = {}
for id, name in pairs(namespaces) do
	id = tonumber(id)
	name = normalize_namespace_name(name)
	namespace_id_to_name[id] = name
	namespace_name_to_id[name] = id
end

local function namespace_id_set(namespace_names)
	local set = {}
	for _, name in ipairs(namespace_names) do
		name = normalize_namespace_name(name)
		local id = namespace_name_to_id[name]
		if not id then
			error(("Namespace name %q not found"):format(name))
		end
		set[id] = true
	end
	return set
end

local function full_title(title, namespace_id)
	local namespace_name = namespace_id_to_name[namespace_id]
	if namespace_name == "" then
		return title
	else
		return namespace_name .. ":" .. title
	end
end

local function print_redirects_in_namespaces(namespace_names)
	local redirect_id_to_title = {}
	local namespaces_to_find = namespace_id_set(namespace_names)
	for page in require "iter_sql" "page" do
		if page.is_redirect and namespaces_to_find[page.namespace] then
			redirect_id_to_title[page.id] = full_title(page.title, page.namespace)
		end
	end

	for redirect in require "iter_sql" "redirect" do
		local from_id = redirect.from
		local from_title = redirect_id_to_title[from_id]
		if from_title then
			local to_title = full_title(redirect.title, redirect.namespace)
			print(from_title, to_title)
			redirect_id_to_title[from_id] = nil
		end
	end

	assert(
		not next(redirect_id_to_title),
		"a redirect page in page.sql was not found in redirect.sql")
end

print_redirects_in_namespaces({ ... })
