// ┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
// ┃ ██████ ██████ ██████       █      █      █      █      █ █▄  ▀███ █       ┃
// ┃ ▄▄▄▄▄█ █▄▄▄▄▄ ▄▄▄▄▄█  ▀▀▀▀▀█▀▀▀▀▀ █ ▀▀▀▀▀█ ████████▌▐███ ███▄  ▀█ █ ▀▀▀▀▀ ┃
// ┃ █▀▀▀▀▀ █▀▀▀▀▀ █▀██▀▀ ▄▄▄▄▄ █ ▄▄▄▄▄█ ▄▄▄▄▄█ ████████▌▐███ █████▄   █ ▄▄▄▄▄ ┃
// ┃ █      ██████ █  ▀█▄       █ ██████      █      ███▌▐███ ███████▄ █       ┃
// ┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
// ┃ Copyright (c) 2017, the Perspective Authors.                              ┃
// ┃ ╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌ ┃
// ┃ This file is part of the Perspective library, distributed under the terms ┃
// ┃ of the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0). ┃
// ┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛

const lightCodeTheme = require("prism-react-renderer/themes/github");
const darkCodeTheme = require("prism-react-renderer/themes/dracula");

const fs = require("fs");

const examples = fs.readdirSync("static/blocks").map((ex) => {
    const files = fs
        .readdirSync(`static/blocks/${ex}`)
        .filter(
            (x) =>
                !x.startsWith(".") &&
                !x.endsWith(".png") &&
                !x.endsWith(".arrow")
        )
        .map((x) => {
            return {
                name: x,
                contents: fs
                    .readFileSync(`static/blocks/${ex}/${x}`)
                    .toString(),
            };
        });

    return {
        name: ex,
        files,
    };
});

/** @type {import('@docusaurus/types').Config} */
const config = {
    title: "Perspective",
    // tagline: "Dinosaurs are cool",
    url: "https://perspective.finos.org",
    baseUrl: "/",
    onBrokenLinks: "warn",
    onBrokenMarkdownLinks: "warn",
    favicon: "https://www.finos.org/hubfs/FINOS/finos-logo/favicon.ico",

    // GitHub pages deployment config.
    // If you aren't using GitHub pages, you don't need these.
    organizationName: "finos", // Usually your GitHub org/user name.
    projectName: "perspective", // Usually your repo name.
    trailingSlash: true,

    customFields: {
        examples,
    },
    // markdown: {
    //     format: "md",
    // },

    // Even if you don't use internalization, you can use this field to set useful
    // metadata like html lang. For example, if your site is Chinese, you may want
    // to replace "en" with "zh-Hans".
    i18n: {
        defaultLocale: "en",
        locales: ["en"],
    },
    plugins: ["./plugins/perspective-loader"],
    presets: [
        [
            "classic",
            /** @type {import('@docusaurus/preset-classic').Options} */
            ({
                docs: false,
                // docs: {
                //     //  sidebarPath: require.resolve("./sidebars.js"),
                //     docItemComponent: require.resolve(
                //         "./src/components/DocItem"
                //     ),
                //     // Please change this to your repo.
                //     // Remove this to remove the "edit this page" links.
                //     // editUrl:
                //     //     "https://github.com/facebook/docusaurus/tree/main/packages/create-docusaurus/templates/shared/",
                // },
                // blog: {
                //     showReadingTime: true,
                //     // Please change this to your repo.
                //     // Remove this to remove the "edit this page" links.
                //     editUrl:
                //         "https://github.com/facebook/docusaurus/tree/main/packages/create-docusaurus/templates/shared/",
                // },
                theme: {
                    customCss: require.resolve("./src/css/custom.css"),
                },
            }),
        ],
    ],

    themeConfig:
        /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
        ({
            colorMode: {
                defaultMode: "dark",
            },
            navbar: {
                // title: "Perspective",
                logo: {
                    alt: "Perspective",
                    src: "svg/perspective-logo-light.svg",
                },
                items: [
                    {
                        type: "dropdown",
                        position: "right",
                        label: "Docs",
                        items: [
                            {
                                type: "html",
                                value: "<span>JavaScript</span>",
                            },
                            {
                                href: "https://docs.rs/perspective-viewer/latest/perspective_viewer/",
                                label: "`@finos/perspective-viewer` JavaScript UI API",
                            },
                            {
                                href: "https://docs.rs/perspective-js/latest/perspective_js/",
                                label: "`@finos/perspective` JavaScript Client/Server API",
                            },
                            {
                                href: "https://docs.rs/perspective-js/latest/perspective_js/struct.Table.html",
                                label: "`Table` API",
                            },
                            {
                                href: "https://docs.rs/perspective-js/latest/perspective_js/struct.View.html",
                                label: "`View` API",
                            },
                            {
                                href: "https://docs.rs/perspective-js/latest/perspective_js/#installation",
                                label: "Installation Guide",
                            },
                            {
                                type: "html",
                                value: "<span>Python</span>",
                            },
                            {
                                href: "https://docs.rs/perspective-python/latest/perspective_python/",
                                label: "`perspective-python` Python Client/Server API",
                            },
                            {
                                href: "https://docs.rs/perspective-python/3.1.0/perspective_python/#perspectivewidget",
                                label: "`PerspectiveWidget` Jupyter Plugin",
                            },
                            {
                                href: "https://docs.rs/perspective-python/latest/perspective_python/struct.Table.html",
                                label: "`Table` API",
                            },
                            {
                                href: "https://docs.rs/perspective-python/latest/perspective_python/struct.View.html",
                                label: "`View` API",
                            },
                            {
                                type: "html",
                                value: "<span>Rust</span>",
                            },
                            {
                                href: "https://docs.rs/perspective/latest/perspective/",
                                label: "`perspective`, Rust API",
                            },
                            {
                                href: "https://docs.rs/perspective-client/latest/perspective_client/struct.Table.html",
                                label: "`Table` API",
                            },
                            {
                                href: "https://docs.rs/perspective-client/latest/perspective_client/struct.View.html",
                                label: "`View` API",
                            },
                            {
                                type: "html",
                                value: "<span>Appendix</span>",
                            },
                            {
                                href: "https://docs.rs/perspective-server/latest/perspective_server/",
                                label: "Data Binding",
                            },
                            {
                                href: "https://docs.rs/perspective-client/latest/perspective_client/config/expressions/",
                                label: "Expression Columns",
                            },
                        ],
                    },
                    {
                        to: "/examples",
                        position: "right",
                        label: "Examples",
                    },
                    {
                        href: "https://www.prospective.co/blog",
                        label: "Blog",
                        position: "right",
                    },
                    {
                        href: "https://github.com/finos/perspective",
                        label: "GitHub",
                        position: "right",
                    },
                ],
            },
            footer: {
                links: [
                    // {
                    //     title: "Docs",
                    //     items: [
                    //         {
                    //             label: "JavaScript User Guide",
                    //             to: "/docs/js",
                    //         },
                    //         {
                    //             label: "Python User Guide",
                    //             to: "/docs/python",
                    //         },
                    //     ],
                    // },
                    // {
                    //     title: "More",
                    //     items: [
                    //         {
                    //             label: "GitHub",
                    //             href: "https://github.com/finos/perspective",
                    //         },
                    //         {
                    //             href: "https://www.prospective.co/blog",
                    //             label: "Blog",
                    //             position: "right",
                    //         },
                    //     ],
                    // },
                ],
                copyright: `Copyright © ${new Date().getFullYear()} The Perspective Authors`,
            },
            prism: {
                theme: lightCodeTheme,
                darkTheme: darkCodeTheme,
            },
        }),
};

module.exports = config;
