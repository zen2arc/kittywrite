-- word count plugin
-- demonstrates the kittywrite plugin api

local kw = plugin({
    name = "word-count",
    version = "1.0.0",
    description = "counts words in current file",
    author = "kittywrite",
})

-- hook into file open to show word count
kw:on("open", function()
    local content = kittywrite.get_content()
    local count = 0
    for _ in content:gmatch("%S+") do
        count = count + 1
    end
    kittywrite.notify("word count: " .. count)
end)

-- command to count words
kw:command("word-count", "count words in current file", function()
    local content = kittywrite.get_content()
    local count = 0
    for _ in content:gmatch("%S+") do
        count = count + 1
    end
    return count .. " words, " .. kittywrite.get_line_count() .. " lines"
end)

-- command that uses more api
kw:command("file-info", "show file info", function()
    local path = kittywrite.get_file_path()
    local name = kittywrite.get_file_name()
    local lines = kittywrite.get_line_count()
    local theme = kittywrite.get_theme()
    local font = kittywrite.get_font_size()
    return string.format("%s | %d lines | theme: %s | font: %.0f", name, lines, theme, font)
end)
