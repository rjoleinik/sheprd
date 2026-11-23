import Foundation
import Testing
@testable import SheprdGitHub
@testable import SheprdRemoteCore

/// The repositories a new project may clone.
struct GitHubRepositoriesTests {
    private let answer = SignInFixtures.json(#"""
    [
      {"name":"weather","owner":{"login":"sheprd-labs"},"language":"Swift","private":true,"pushed_at":"2026-11-20T10:00:00Z"},
      {"name":"bad name","owner":{"login":"acme"}},
      {"name":"dotfiles","owner":{"login":"penso"},"language":null,"private":false}
    ]
    """#)

    @Test func theAccountsRepositoriesRecentFirstWithPlainNames() async throws {
        let transport = FakeTransport([answer])
        let api = GitHubAPI(transport: transport.transport)
        let found = try await api.repositories(host: GitHubFixtures.host, token: GitHubToken("ghu_a1"))
        #expect(found.map(\.fullName) == ["sheprd-labs/weather", "penso/dotfiles"])
        #expect(found.first?.isPrivate == true)
        #expect(found.first?.language == "Swift")
        #expect(found.first?.pushedAt != nil)
        #expect(found.last?.name == "dotfiles")
        let url = try #require(transport.requests.first?.url)
        #expect(url.path() == "/user/repos")
        #expect(url.query()?.contains("sort=pushed") == true)
    }

    @Test func anAnswerThatIsNotAListIsMalformed() async throws {
        let api = GitHubAPI(transport: FakeTransport([SignInFixtures.json(#"{"message":"no"}"#)]).transport)
        await #expect(throws: GitHubError.malformed) {
            try await api.repositories(host: GitHubFixtures.host, token: GitHubToken("ghu_a1"))
        }
    }
}
