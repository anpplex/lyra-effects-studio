import Darwin
import LyraEffectsCLI

let exitCode = CLI.run(arguments: Array(CommandLine.arguments.dropFirst()))
exit(exitCode)
