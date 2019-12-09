local language_names = require "mediawiki.languages.name_to_code"

local Array = require "mediawiki.array"
local titles_by_language_code = {}
local titles_by_language_code = setmetatable({}, {
	__index = function(self, language_code)
		local titles = Array()
		self[language_code] = titles
		return titles
	end,
	__gc = function(self)
		local ok, err = pcall(function()
			local comp = require "casefold".comp
			for language_code, titles in pairs(self) do
				local file = assert(io.open(language_code .. ".txt", "wb"))
				titles:sort(comp)
				file:write(titles:concat "\n")
				file:close()
			end
		end)
		if not ok then io.stderr:write(err, "\n") end
	end,
})

local count = 0
local limit = ... and tonumber(...) or math.maxinteger

return function(header, title)
	local language_name
	local prefix = title:match "^[^:]+"
	if prefix == "Reconstruction" or prefix == "Appendix" then
		language_name = title:match "^[^:]+:([^/]+)"
	elseif header.level == 2 then
		language_name = header.text
	end
	
	local language_code = language_name and language_names[language_name]
	if language_code then
		titles_by_language_code[language_code]:insert(title)
	end
	
	count = count + 1
	return count < limit
end