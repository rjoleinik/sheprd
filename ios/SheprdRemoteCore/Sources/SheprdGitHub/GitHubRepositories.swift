import Foundation
import SheprdRemoteCore

/// A repository the signed-in account can clone, for a new project.
public struct GitHubRepository: Hashable, Sendable, Identifiable {
    /// `owner/name`, as the computer is asked to clone it.
    public var fullName: String
    public var language: String?
    public var isPrivate: Bool
    public var pushedAt: Date?

    public init(fullName: String, language: String? = nil, isPrivate: Bool = false, pushedAt: Date? = nil) {
        self.fullName = fullName
        self.language = language
        self.isPrivate = isPrivate
        self.pushedAt = pushedAt
    }

    public var id: String { fullName.lowercased() }
    /// The folder a clone gets by default.
    public var name: String { fullName.split(separator: "/").last.map(String.init) ?? fullName }

    /// Repositories listed at most: one page, recently pushed first, as
    /// each answer is several kilobytes.
    static let limit = 50
    static let path = "/user/repos?sort=pushed&per_page=\(limit)&affiliation=owner,collaborator,organization_member"

    /// GitHub's answer, skipping repositories whose names are not plain.
    static func parse(_ data: Data) throws -> [GitHubRepository] {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        let answers: [Answer]
        do { answers = try decoder.decode([Answer].self, from: data) } catch { throw GitHubError.malformed }
        return answers.compactMap { answer in
            guard let owner = PullRequestSearchAnswer.name(answer.owner?.login),
                  let name = PullRequestSearchAnswer.name(answer.name)
            else { return nil }
            return GitHubRepository(
                fullName: "\(owner)/\(name)",
                language: answer.language,
                isPrivate: answer.private ?? false,
                pushedAt: answer.pushedAt
            )
        }
        .prefix(limit).map { $0 }
    }

    struct Answer: Decodable {
        struct Owner: Decodable { var login: String? }
        var name: String?
        var owner: Owner?
        var language: String?
        var `private`: Bool?
        var pushedAt: Date?

        enum CodingKeys: String, CodingKey {
            case name, owner, language, `private`
            case pushedAt = "pushed_at"
        }
    }
}

/// What reading the account's repositories came to.
public enum RepositoriesRead: Hashable, Sendable {
    case found([GitHubRepository])
    /// No github.com token, as for the pull request lists.
    case noAccount
    case failed
}

extension GitHubAPI {
    func repositories(host: GitHubHost, token: GitHubToken) async throws -> [GitHubRepository] {
        try GitHubRepository.parse(try await rest(GitHubRepository.path, host: host, token: token))
    }
}

extension PullRequestReader {
    /// The github.com account's repositories, recently pushed first, with
    /// the phone's own sign-in first, then a lent token.
    public func repositories(source: Lenders) async -> RepositoriesRead {
        let host = Self.listHost
        guard await hasAccount(for: host, source: source) else { return .noAccount }
        let found = await attempt(host: host, source: source) { api, token in
            try await api.repositories(host: host, token: token)
        }
        return found.map(RepositoriesRead.found) ?? .failed
    }
}
