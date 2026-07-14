import Foundation

public struct PackDiagnostic: Codable, Equatable, Sendable {
    public enum Severity: String, Codable, Sendable { case error, warning }
    public let severity: Severity
    public let code: String
    public let path: String?
    public let message: String

    public init(severity: Severity = .error, code: String, path: String? = nil, message: String) {
        self.severity = severity; self.code = code; self.path = path; self.message = message
    }
}

public struct PackValidationBudget: Equatable, Sendable {
    public var maximumFileBytes: Int
    public var maximumTotalBytes: Int

    public init(maximumFileBytes: Int = 5 * 1_024 * 1_024, maximumTotalBytes: Int = 20 * 1_024 * 1_024) {
        self.maximumFileBytes = maximumFileBytes
        self.maximumTotalBytes = maximumTotalBytes
    }
}

public struct PackValidator {
    public let budget: PackValidationBudget
    private let fileManager: FileManager

    public init(budget: PackValidationBudget = .init(), fileManager: FileManager = .default) {
        self.budget = budget
        self.fileManager = fileManager
    }

    public func validate(at root: URL) throws -> [PackDiagnostic] {
        let root = root.standardizedFileURL.resolvingSymlinksInPath()
        var diagnostics: [PackDiagnostic] = []
        let manifestURL = root.appending(path: "lyra-pack.json")
        guard fileManager.fileExists(atPath: manifestURL.path) else {
            return [.init(code: "manifest.missing", path: "lyra-pack.json", message: "Pack root must contain lyra-pack.json")]
        }

        let manifest: PackManifest
        do {
            manifest = try CanonicalJSON.decode(PackManifest.self, from: Data(contentsOf: manifestURL))
        } catch {
            return [.init(code: "manifest.invalid", path: "lyra-pack.json", message: String(describing: error))]
        }

        if manifest.id.wholeMatch(of: /[a-z0-9]+(?:\.[a-z0-9][a-z0-9-]*)+/) == nil {
            diagnostics.append(.init(code: "manifest.idInvalid", path: "lyra-pack.json", message: "Pack id must be a lowercase reverse-domain identifier"))
        }

        if manifest.kind == .theme {
            if manifest.entry.style == nil {
                diagnostics.append(.init(code: "theme.styleMissing", path: "lyra-pack.json", message: "Theme Pack requires entry.style"))
            }
            if manifest.entry.html != nil || manifest.capabilities.contains(where: { ["script", "html", "network"].contains($0) }) {
                diagnostics.append(.init(code: "theme.scriptForbidden", path: "lyra-pack.json", message: "Theme Pack cannot declare script, HTML, or network capabilities"))
            }
        }

        let declaredPaths = [manifest.entry.style, manifest.entry.html, manifest.parameters, manifest.integrity, manifest.license.notice]
            .compactMap { $0 } + manifest.scenarios
        for path in declaredPaths {
            diagnostics.append(contentsOf: validateDeclaredPath(path, root: root))
        }

        guard let enumerator = fileManager.enumerator(
            at: root,
            includingPropertiesForKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey],
            options: [.skipsHiddenFiles]
        ) else { return diagnostics }

        var totalBytes = 0
        for case let url as URL in enumerator {
            let relative = String(url.path.dropFirst(root.path.count + 1))
            let values = try url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey])
            if values.isSymbolicLink == true {
                let target = url.resolvingSymlinksInPath().standardizedFileURL
                if !isDescendant(target, of: root) {
                    diagnostics.append(.init(code: "symlink.escapesRoot", path: relative, message: "Symlink target is outside Pack root"))
                }
                continue
            }
            guard values.isRegularFile == true else { continue }

            let size = values.fileSize ?? 0
            totalBytes += size
            if size > budget.maximumFileBytes {
                diagnostics.append(.init(code: "budget.fileExceeded", path: relative, message: "File exceeds the Pack per-file budget"))
            }
            let extensionName = url.pathExtension.lowercased()
            if manifest.kind == .theme, ["js", "mjs", "cjs", "wasm"].contains(extensionName) {
                diagnostics.append(.init(code: "theme.scriptForbidden", path: relative, message: "Theme Pack contains executable web code"))
            }
            let permissions = (try? fileManager.attributesOfItem(atPath: url.path)[.posixPermissions] as? NSNumber)?.intValue ?? 0
            if permissions & 0o111 != 0 {
                diagnostics.append(.init(code: "file.executableForbidden", path: relative, message: "Pack files cannot be executable"))
            }
        }
        if totalBytes > budget.maximumTotalBytes {
            diagnostics.append(.init(code: "budget.totalExceeded", message: "Pack exceeds the total uncompressed size budget"))
        }

        return diagnostics.sorted { ($0.path ?? "", $0.code) < ($1.path ?? "", $1.code) }
    }

    private func validateDeclaredPath(_ path: String, root: URL) -> [PackDiagnostic] {
        guard !path.isEmpty, !path.hasPrefix("/"), !path.split(separator: "/").contains("..") else {
            return [.init(code: "path.traversal", path: path, message: "Pack paths must remain relative to Pack root")]
        }
        let candidate = root.appending(path: path).standardizedFileURL
        guard isDescendant(candidate, of: root) else {
            return [.init(code: "path.traversal", path: path, message: "Pack path resolves outside Pack root")]
        }
        guard fileManager.fileExists(atPath: candidate.path) else {
            return [.init(code: "file.missing", path: path, message: "Declared Pack file does not exist")]
        }
        return []
    }

    private func isDescendant(_ candidate: URL, of root: URL) -> Bool {
        candidate.path == root.path || candidate.path.hasPrefix(root.path + "/")
    }
}
