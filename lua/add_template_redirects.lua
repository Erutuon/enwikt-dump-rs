#! /usr/bin/env lua53

local val_format = ... or "%s.cbor"

local template_whitespace = "[%s_]"
local trim_pattern = "^" .. template_whitespace .. "*(.-)" .. template_whitespace .. "*$"
local function normalize_template_name(template_name)
	return (template_name:gsub(trim_pattern, "%1"):gsub(template_whitespace .. "+", "_"))
end

local function make_template_to_val_map(text, make_val)
	local map = {}
	for line in text:gmatch "[^\n]+" do
		local template_name, val = line:match "^([^\t])\t(.+)$"
		if not template_name then
			template_name = line
			val = make_val(template_name)
		end
		template_name = normalize_template_name(template_name)
		map[template_name] = val
	end
	return map
end

local function filter(arr, func)
	local new_arr = {}
	for _, val in ipairs(arr) do
		if func(val) then
			table.insert(new_arr, val)
		end
	end
	return new_arr
end

local function set_fields_to_val(t, val, ...)
	local keys = {...}
	if type(val) == "function" then
		for _, key in ipairs(keys) do
			t[key] = val(key)
		end
	else
		for _, key in ipairs(keys) do
			t[key] = val
		end
	end
end

local function add_redirects(text, make_val)
	local map = make_template_to_val_map(text, make_val)

	-- If the redirect target is in `map`, use its value for all its redirects.
	-- Otherwise, use the value of the redirect that is first alphabetically
	-- for the redirect target and for all the other redirects that do not have their own values.
	local new_map = {}
	-- local redirects_by_target = require "template_redirects"
	local redirects_by_target = require "cjson".decode(assert(io.open "template_redirects.json"):read "a")
	for target, redirects in pairs(redirects_by_target) do
		if map[target] then
			set_fields_to_val(new_map, map[target], target, table.unpack(redirects))
		else
			local matches = filter(redirects, function(redirect) return map[redirect] ~= nil end)
			
			if #matches > 0 then
				set_fields_to_val(
					new_map,
					function(key) return map[key] or map[matches[1]] end,
					target, table.unpack(redirects))
			end
		end
	end
	
	for template, val in pairs(map) do
		if not new_map[template] then
			new_map[template] = val
		end
	end
	
	return new_map
end

local to_print = {}
for template, val in pairs(add_redirects(
		io.read "a",
		function(template_name)
			return val_format:format((template_name:gsub("[ /]", "_")))
		end)) do
	table.insert(to_print, { template = template, val = val })
end

table.sort(to_print, function(a, b)
	a, b = a.template, b.template
	local a_lower, b_lower = a:lower(), b:lower()
	if a_lower ~= b_lower then
		return a_lower < b_lower
	else
		return a < b
	end
end)

for _, v in ipairs(to_print) do
	print(v.template, v.val)
end

--[[
for template, val in pairs(map) do
	if not new_map[template] then
		new_map[template] = val
	end
	local redirects = redirects_by_target[template]
	-- If this is a redirect target, then let all its redirects have the same
	-- value associated with them, overriding any separate value that
	-- was assigned to them.
	if redirects then
		for _, redirect in ipairs(redirects) do
			new_map[redirect] = val
		end
	end
end
--]]
