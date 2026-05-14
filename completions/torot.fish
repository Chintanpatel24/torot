complete -c torot -f

# Commands
complete -c torot -n "test (count __fish_complete_subcommand_args) = 0" -a tools -d "List detected security tools"
complete -c torot -n "test (count __fish_complete_subcommand_args) = 0" -a scan -d "Run a security scan against a target"
complete -c torot -n "test (count __fish_complete_subcommand_args) = 0" -a report -d "Generate a report for a session"
complete -c torot -n "test (count __fish_complete_subcommand_args) = 0" -a config -d "Show current configuration"
complete -c torot -n "test (count __fish_complete_subcommand_args) = 0" -a help -d "Show help information"

# Scan options
complete -c torot -n "__fish_seen_subcommand_from scan" -s t -l target -d "Target URL, host, or path" -r
complete -c torot -n "__fish_seen_subcommand_from scan" -l tools -d "Comma-separated tool list" -r
complete -c torot -n "__fish_seen_subcommand_from scan" -s m -l mode -d "Scan mode" -xa "single deep passive"
complete -c torot -n "__fish_seen_subcommand_from scan" -s o -l output -d "Report output path" -r
complete -c torot -n "__fish_seen_subcommand_from scan" -l template-file -d "Custom report template" -r

# Report options
complete -c torot -n "__fish_seen_subcommand_from report" -s s -l session -d "Session ID" -r
complete -c torot -n "__fish_seen_subcommand_from report" -s o -l output -d "Report output path" -r
complete -c torot -n "__fish_seen_subcommand_from report" -l template-file -d "Custom report template" -r
