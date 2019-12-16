local lpeg = require "lpeg"

for k, v in pairs(lpeg) do
	if type(k) == "string" and k:find "^%u" then
		_ENV[k] = v
	end
end

local type_patterns = {
	string = P "'" * Cs((P "\\" / "" * (P "'" / "'" + P "\\" / "\\") + (1 - P "'"))^0) * P "'",
	float = (S "+-"^-1 * R "09"^1 * "." * R "09"^1) / tonumber,
	integer = R "09"^1 / tonumber,
	boolean = S "01" / function(num) return num == "1" end,
}

local NULL = function() end -- yuck
local null_pattern = P "NULL" / function() return NULL end
local function make_nullable_pattern(type_patterns, nullable_type)
	local non_nullable_type = nullable_type:gsub("^nullable_", "")
	local non_nullable_pattern = type_patterns[non_nullable_type]
	if not non_nullable_pattern then
		error("Cannot create pattern for " .. nullable_type
			.. " because there is no pattern for " .. non_nullable_type)
	end
	local nullable_pattern = non_nullable_pattern + null_pattern
	type_patterns[nullable_type] = nullable_pattern
	return nullable_pattern
end

setmetatable(type_patterns, {
	__index = function(self, k)
		if type(k) == "string" then
			if k:find "^nullable_" then
				return make_nullable_pattern(type_patterns, k)
			else
				error("No type pattern for " .. k)
			end
		else
			error("Expected string, got " .. type(k))
		end
	end,
})

for k, v in pairs {
	category_id = "integer",
	page_id = "integer",
	page_namespace = "integer",
	page_title = "string", -- string 255 bytes long
	page_type = "string",
	rev_id = "nullable_integer",
	timestamp = "string",
	user_group = "string",
	user_id = "integer",
} do
	type_patterns[k] = type_patterns[v]
end

local function unpack_ipairs(t)
	local i = 0
	return function()
		i = i + 1
		local val = t[i]
		if val then
			return i, table.unpack(val)
		end
	end
end

local function make_value_pattern(name_prefix, args)
	local pattern = P "("
	for i, field_name, type_name in unpack_ipairs(args) do
		local subpattern = Cg(type_patterns[type_name], field_name)
		if i ~= 1 then
			pattern = pattern * "," * subpattern
		else
			pattern = pattern * subpattern
		end
	end
	return pattern * ")"
end

local function find_end_of_first_match(pattern)
	return P {
		(1 - V "pattern")^0 * V "pattern" * Cp(),
		pattern = P(pattern),
	}
end

local function make_array_of_string_duples(str)
	local array = {}
	for line in str:gsub("%s*%-%-[^\n]*", ""):gmatch "[^\n]+" do
		local word1, word2 = line:match "^(%S+) (%S+)$"
		if not word1 then
			error("line " .. line .. " doesn't match pattern.")
		end
		table.insert(array, { word1, word2 })
	end
	return array
end

local debug = false
local function make_iter(table_name, name_prefix, args)
	-- name_prefix is included for completeness, but is not used here.
	local value_pattern = make_value_pattern(name_prefix, make_array_of_string_duples(args))
	local insert_into = "INSERT INTO `" .. table_name .. "` VALUES "
	local prefix_pattern = find_end_of_first_match(insert_into)
	local value_comma_pattern = Ct(value_pattern) * P ","^-1 * Cp()
	return function(str)
		local pos = 1
		local debug_count = 0
		return function()
			while true do
				local values, new_pos = value_comma_pattern:match(str, pos)
				if values then
					pos = new_pos
					return values
				else
					new_pos = prefix_pattern:match(str, pos)
					if new_pos then
						if debug then
							local skipped_range = str:sub(pos, new_pos - #insert_into - 1)
							if not skipped_range:find "^;%s+$" then
								io.stderr:write(skipped_range)
								debug_count = debug_count + 1
								if debug_count > 10 then
									error("too much debugging")
								end
							end
						end
						pos = new_pos
					else
						pos = #str
						break
					end
				end
			end
		end
	end
end

local function inspect_result(database_table)
	local iter = M[database_table] or error("Unknown table " .. tostring(database_table))
	local file = assert(io.open(database_table .. ".sql", "rb"))
	local sample = file:read "a"-- (2^14)
	for values in require "itertools".islice(iter(sample), 1, 10) do
		print(require 'inspect'(values))
	end
end

return { make_iter = make_iter, NULL = NULL }
