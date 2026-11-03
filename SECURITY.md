# Security

Please report security vulnerabilities privately through [GitHub's security advisory form](https://github.com/itIsMaku/clickup-tray-time-tracker/security/advisories/new).

Include the affected version, operating system, reproduction steps, and likely impact. Do not post API tokens or private task/workspace data in public issues. This is a community-maintained project without a guaranteed response time.

The application stores its personal API token in the operating system credential store and sends it directly to `api.clickup.com`. Preferences contain workspace IDs, recent task names, goals, and language. They are not a credential store.

Only the latest released version is maintained. If you discover a problem on an older build, check whether it is still reproducible on the current version when practical.
