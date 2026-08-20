# Private Sorcery card/rules data and artwork: current legal research

**Research date:** 2026-08-20  
**Intended facts:** one owner in Alaska; local and noncommercial simulator; no hosted service; the owner personally runs any acquisition script; official public card/rules sources; possible local display of card artwork.  
**Not legal advice:** this is issue-spotting and risk analysis, not a legal opinion. Copyright ownership, registrations, site presentation at the moment of assent, the user’s account history, and New Zealand law can change the result. U.S./Alaska and New Zealand counsel should review any launch, redistribution, commercial use, or access after an objection.

## Reading key

- **Law** — statute or holding of a court. Binding weight is identified where material.
- **Contract text** — what a posted agreement says, without assuming it was formed or is enforceable.
- **Publisher statement** — operational guidance on an official page; it may be permission, evidence of permission, or merely information depending on context.
- **Inference** — application of those authorities to the assumed facts.
- **Uncertainty** — an issue the public record does not resolve.

## Bottom line

The lowest-risk workable architecture is a content-free simulator repository that implements game mechanics in original TypeScript, accepts a user-supplied local official snapshot, keeps that snapshot ignored and private, omits artwork by default, never contacts an image CDN, and stops all automated acquisition until Erik’s Curiosa gives written clarification.

That conservative boundary is driven more by the publisher’s current contract language than by the abstract copyrightability of game mechanics:

1. **Publisher statement:** the official [Sorcery public API page](https://api.sorcerytcg.com/) says developers may access card data, recommends intermittent polling and diffing, and recommends self-hosting required data.
2. **Contract text:** the official [Terms & Conditions](https://sorcerytcg.com/terms), last updated January 15, 2024, prohibit automated/non-human access, scripts, data-mining tools, and systematic retrieval that creates a collection or database without written permission.
3. **Publisher statement:** the API page says card images are not in the API, forbids using its private CDN to serve images, and links a [public Google Drive image folder](https://drive.google.com/drive/folders/17IrJkRGmIU9fDSTU2JQEU9JlFzb5liLJ?usp=sharing). It does not state an express bulk-download, reuse, or redistribution license for the images.
4. **Operational fact:** on 2026-08-20, the API host’s [`robots.txt`](https://api.sorcerytcg.com/robots.txt) returned `User-agent: *` / `Disallow: /`, while the [`/api/cards`](https://api.sorcerytcg.com/api/cards) endpoint remained public and rate-limited. A `HEAD` request returned HTTP 200, `Content-Type: application/json`, `ETag`, and `X-Ratelimit-Limit: 3`. A robots directive is not by itself a copyright license, contract, access gate, or statute, but it reinforces the need to resolve the contradictory automation signals.

**Inference:** one private, noncommercial copy of functional card/rules data for analysis has a materially better copyright fair-use position than a public or commercial product. It also creates no public-display or distribution event. But “private” is not a general exemption from the reproduction right, and a user-run script is still a script. The user’s personal execution changes who makes the network request and copy; it does not make an otherwise prohibited automated request human or authorized.

**Inference:** artwork is materially riskier. It is highly creative, the simulator does not need it to reproduce rules outcomes, and full-size local display generally serves the same visual purpose as the original card image. Keeping it private reduces exposure and market harm, but does not remove the act of reproduction. The public Drive link is encouraging evidence for ordinary personal access, not a clear license to bulk-copy all art.

## 1. Current official-source audit

### 1.1 Erik’s Curiosa Terms

The [Terms](https://sorcerytcg.com/terms) identify Erik’s Curiosa LLC as the counterparty, state a New Zealand registration/address, cover `sorcerytcg.com`, `play.sorcerytcg.com`, `curiosa.io`, and related or linked media, and say access constitutes agreement. They choose New Zealand law and contain informal-negotiation and arbitration language, with exceptions for intellectual-property, piracy, unauthorized-use, and injunctive-relief disputes.

The clauses most relevant here are:

- **Contract text:** site source code, databases, functionality, text, photographs, and graphics are claimed as owned, controlled, or licensed content protected by intellectual-property laws.
- **Contract text:** content is supplied for information and personal use. The Terms grant an eligible user a limited license to access the Site and download or print a copy of a portion properly accessed, solely for personal, noncommercial use.
- **Contract text:** the user represents that the user will not access through automated or non-human means, including a bot or script.
- **Contract text:** systematic retrieval to build a collection, compilation, database, or directory requires written permission.
- **Contract text:** automated use, data-mining, robots, similar extraction tools, spiders, scrapers, offline readers, and unauthorized scripts are prohibited, subject to an exception for standard search-engine or browser use.
- **Contract text:** bypassing access restrictions or security/copy controls is prohibited.
- **Contract text:** commercial endeavors are prohibited unless endorsed or approved.
- **Contract text:** contact for information about use is `community@sorcerytcg.com`.

**Uncertainty:** the API is a `sorcerytcg.com` subdomain and is plainly related to the covered sites, but the Terms do not name `api.sorcerytcg.com`. The public API page contains no visible Terms link or separate dated API license in the text retrieved. Whether the general Terms legally cover a particular anonymous API request depends on scope, notice, assent, and governing-law analysis.

**Uncertainty:** the personal-use download grant is meaningful but its phrase “a copy of any portion” is not a clear authorization to download the entire card database or complete art library. The API-specific self-hosting instruction is more specific, but the public record does not establish when it was posted or whether it modifies the January 2024 Terms.

### 1.2 Official API and image guidance

The official [API landing page](https://api.sorcerytcg.com/) currently says:

- **Publisher statement:** it is a “Public API” that allows developers to access card data.
- **Publisher statement:** it may change without notice and is rate-limited per user.
- **Publisher statement:** external applications should not interact with the data live; developers should poll intermittently, diff against the previous import, and host required data themselves.
- **Publisher statement:** images are excluded from API output.
- **Publisher statement:** others may not use the private CDN to serve images.
- **Publisher statement:** released card images are in the linked public Drive folder and filenames can be derived from the variant slug.

**Inference:** the polling/diffing/self-hosting language is strong evidence of permission for some automated card-data acquisition and persistent local caching. It is also in direct tension with the Terms and the API host’s current robots directive. It is not prudent to decide unilaterally that the API page overrides the Terms; obtain written confirmation.

**Inference:** “public API” means publicly reachable and developer-facing. It does not by itself mean public domain, Creative Commons, or unrestricted redistribution. The API page recommends hosting *required* data, not republishing the entire official database.

**Inference:** never derive or probe private-CDN URLs, hotlink them, reuse captured tokens, or proxy them. Use of files the publisher deliberately placed in the public Drive is a different access posture, but image copyright and contract questions remain.

### 1.3 SadKingLabs boundary

The current [SadKingLabs Sorcery Registry README](https://github.com/sadkinglabs/sorcery-registry) describes stable `codex_id` and `printing_id` identifiers, a deterministic JSON export, a schema, corrections, slug history, and a sync pipeline fed by the official API. The repository says it includes no images and mirrors official attributes, with documented corrections.

The [repository LICENSE](https://github.com/sadkinglabs/sorcery-registry/blob/main/LICENSE) contains the MIT License for the repository’s code, copyright 2026 SadKingLabs, followed by an express boundary: Sorcery, card names, card text, and related data remain Erik’s Curiosa’s property; the project claims no ownership over that republished API material. The README similarly says code is MIT-licensed while card data belongs to Erik’s Curiosa.

Consequences:

- **Contract/license text:** MIT permission clearly covers SadKingLabs code and associated software documentation, subject to preserving the notice.
- **Uncertainty:** the text does not clearly license the JSON dataset as a whole under MIT. Do not treat the repository’s public availability or MIT badge as an Erik’s Curiosa content license.
- **Inference:** independently run MIT-licensed transformation/registry code against a separately authorized official snapshot. Preserve SadKingLabs notices for copied code.
- **Uncertainty:** stable identifiers, schema choices, history, corrections, and checksums include SadKingLabs contributions. Some may be uncopyrightable facts, methods, or short labels; the compilation or explanatory material may be protected. Ask SadKingLabs to confirm a license for those non-Erik contributions and for the export structure if they will be vendored or redistributed.
- **Law:** a downstream license cannot grant rights the licensor does not own. SadKingLabs correctly disclaims ownership of the official card content; Erik’s Curiosa is the permission source for that content.

## 2. Copyright: what is and is not likely protected

### 2.1 Governing rules

**Law (federal statute):** copyright protects original works fixed in a medium, including literary and pictorial works, but not an idea, procedure, process, system, method of operation, concept, principle, or discovery. [17 U.S.C. §§ 102–103](https://www.copyright.gov/title17/92chap1.html).

**Law (U.S. Supreme Court):** copyright in a work explaining a system does not grant an exclusive right to use the system itself; that functional monopoly belongs, if anywhere, to patent rather than copyright law. *Baker v. Selden*, 101 U.S. 99, 102–05 (1880), [official U.S. Reports PDF](https://tile.loc.gov/storage-services/service/ll/usrep/usrep101/usrep101099/usrep101099.pdf).

**Law (U.S. Supreme Court):** facts are not copyrightable; an original selection or arrangement of facts can receive only compilation protection, and effort alone is insufficient. *Feist Publications, Inc. v. Rural Telephone Service Co.*, 499 U.S. 340, 344–64 (1991), [official U.S. Reports PDF](https://www.govinfo.gov/content/pkg/USREPORTS-499/pdf/USREPORTS-499-340.pdf). The Ninth Circuit describes factual-compilation protection as thin and generally requires virtual identity in the protected selection/arrangement. *Experian Information Solutions, Inc. v. Nationwide Marketing Services, Inc.*, 893 F.3d 1176, 1181–88 (9th Cir. 2018), [opinion](https://law.justia.com/cases/federal/appellate-courts/ca9/16-16987/16-16987-2018-06-27.html).

**Law (Copyright Office guidance):** the idea, title, and methods for playing a game are not protected; sufficiently expressive rules text and graphic art can be. [Copyright Office, “Games”](https://www.copyright.gov/register/tx-games.html). Names, titles, slogans, and short phrases generally are not copyrightable, though trademark law can separately apply. [Copyright Office Circular 33](https://www.copyright.gov/circs/circ33.pdf).

**Law (binding Ninth Circuit):** a functional sequence or system remains excluded by § 102(b), while the words and pictures that describe it may be protected. *Bikram’s Yoga College of India, L.P. v. Evolation Yoga, LLC*, 803 F.3d 1032, 1037–42 (9th Cir. 2015), [opinion](https://law.justia.com/cases/federal/appellate-courts/ca9/13-55763/13-55763-2015-10-08.html). Playing a game is not a statutory public performance. *Allen v. Academic Games League of America, Inc.*, 89 F.3d 614, 616–18 (9th Cir. 1996), [opinion](https://law.justia.com/cases/federal/appellate-courts/F3/89/614/582706/).

### 2.2 Applied to Sorcery materials

| Material | Likely copyright treatment | Practical treatment |
|---|---|---|
| Game mechanics, turn structure, legal-action relationships, formulas | Functional system/method under § 102(b); not protected as such | Independently implement behavior in original code. Do not copy expressive explanations or diagrams unnecessarily. |
| Numeric stats and classifications (mana, threshold, power, rarity, set, finish) | Many are functional parameters or bibliographic facts. But authored estimates/invented values are not automatically “facts”; the Ninth Circuit held authored price estimates could be original creations in *CDN Inc. v. Kapes*, 197 F.3d 1256, 1259–62 (9th Cir. 1999), [opinion](https://caselaw.findlaw.com/court/us-9th-circuit/1082116.html). | Treat the full set as potentially protected or licensed, even if individual values have weak protection. Store only fields the engine needs. |
| Card names, set names, ordinary keywords | Often too short for copyright, but may be source-identifying trademarks or part of a larger expressive work | Nominative internal identification is lower risk; avoid logos and any suggestion of sponsorship. |
| Rules text on a card | Functional instructions may merge with the mechanic or have thin protection; exact phrasing can still contain original literary expression | Prefer structured mechanics and original paraphrase in UI. Retain exact official text only in the private authority snapshot when fidelity/provenance requires it. |
| Flavor text, lore, characters | Creative literary/fictional expression; invented “facts” are not safely treated like real-world facts | Exclude unless written permission is obtained. They are not needed for simulation. |
| Rulebook prose and diagrams | Mechanics are unprotected; prose, examples, arrangement, and diagrams can be protected | Keep the lawfully downloaded rulebook private as authority. Implement rules independently and cite rule sections; do not ship the PDF or reproduce lengthy prose. |
| Card artwork, card frames, symbols, scans | Strong pictorial/graphic expression; a full card image may combine several protected elements and marks | Exclude by default. Permission should expressly cover local copies and in-simulator display. |
| API/database organization | Raw facts are unprotected; creative selection/coordination/arrangement may receive thin compilation protection | Do not clone the publisher’s presentation. Normalize only authorized, necessary fields into an independently designed schema. |

**Uncertainty:** no located reported U.S. case decides copyright in Sorcery card text or this particular API schema. Classification is field- and expression-specific. Calling all card metadata “facts” is too broad because some content represents creative game design or fictional expression rather than observations about the real world.

## 3. Copying, caching, and private display

**Law:** the copyright owner has the exclusive rights to reproduce the work, prepare derivatives, distribute copies publicly, and publicly display covered works, subject to statutory limitations. [17 U.S.C. § 106](https://www.copyright.gov/title17/92chap1.html). Downloading a persistent file is a reproduction even if no one else receives it.

**Law (binding Ninth Circuit):** ownership of a lawful physical copy permits disposition of that particular copy under § 109, not reproduction into a new digital copy. *Disney Enterprises, Inc. v. VidAngel, Inc.*, 869 F.3d 848, 856–57 (9th Cir. 2017), [opinion](https://law.justia.com/cases/federal/appellate-courts/ca9/16-56843/16-56843-2017-08-24.html). Thus, owning a physical Sorcery card does not automatically authorize scanning it.

**Law:** display is “public” when it occurs in a public place, to a substantial group outside a normal family/social circle, or is transmitted to the public. [17 U.S.C. § 101](https://www.copyright.gov/title17/92chap1.html). A purely local display to the owner is ordinarily not a public display. That does not excuse the copy used to make the display.

**Law:** there is no general U.S. “private copy” exception for images, books, or game assets. *Sony Corp. of America v. Universal City Studios, Inc.*, 464 U.S. 417, 447–56 (1984), found the specific private, noncommercial time-shifting of freely broadcast television fair on its record; it did not establish that every home copy is fair. [Official U.S. Reports PDF](https://tile.loc.gov/storage-services/service/ll/usrep/usrep464/usrep464417/usrep464417.pdf). *VidAngel* later stressed that format- or space-shifting is not a categorical § 107 defense and rejected a service’s copying of entire films.

**Inference:** ordinary browser rendering and transient cache files used to view an authorized public page have the strongest implied-authorization posture. “Save image as,” screenshots, scans, or a script that creates a durable library are additional reproductions and should rest on an express/implied license or fair use, not on the fact that the image could be viewed.

**Inference:** local caching of the API JSON is better supported because the API page specifically recommends self-hosting required data. The unresolved automation/Terms conflict remains. Local caching of art has no equivalent express self-hosting instruction.

## 4. Fair use for the private simulator

### 4.1 Legal framework

**Law:** fair use requires case-specific balancing of purpose/character, nature of the work, amount/substantiality, and market effect. Research is an illustrative purpose, not an automatic exemption. [17 U.S.C. § 107](https://www.copyright.gov/title17/92chap1.html); [Copyright Office fair-use guidance](https://www.copyright.gov/fair-use/more-info.html).

**Law (U.S. Supreme Court):** noncommercial status is relevant but not dispositive. The court must compare the specific challenged use’s purpose with the original use and consider substitution/licensing markets. *Andy Warhol Foundation for the Visual Arts, Inc. v. Goldsmith*, 598 U.S. 508, 525–40 (2023), [opinion](https://www.supremecourt.gov/opinions/22pdf/21-869_87ad.pdf).

**Law (U.S. Supreme Court):** copying functional API material needed to let programmers use existing skills in a new computing environment was fair on the particular record. The Court assumed copyrightability and emphasized the functional nature, new program, amount needed, and market effects. *Google LLC v. Oracle America, Inc.*, 593 U.S. 1, 20–40 (2021), [opinion](https://www.supremecourt.gov/opinions/20pdf/18-956_d18f.pdf).

**Law (binding Ninth Circuit):** intermediate copying can be fair when it is necessary to reach unprotected functional elements for a legitimate compatibility purpose. *Sega Enterprises Ltd. v. Accolade, Inc.*, 977 F.2d 1510, 1518–27 (9th Cir. 1992), [opinion](https://law.justia.com/cases/federal/appellate-courts/F2/977/1510/305345/); *Sony Computer Entertainment, Inc. v. Connectix Corp.*, 203 F.3d 596, 602–08 (9th Cir. 2000), [opinion](https://law.justia.com/cases/federal/appellate-courts/F3/203/596/474793/). Those cases do not create a general “simulator” exemption; necessity and the content of the finished product matter.

**Law (binding Ninth Circuit):** reduced-resolution image copies used as a search index were highly transformative and fair on the record. *Perfect 10, Inc. v. Amazon.com, Inc.*, 508 F.3d 1146, 1163–68 (9th Cir. 2007), [official amended opinion](https://cdn.ca9.uscourts.gov/datastore/opinions/2007/12/03/0655405.pdf). Full-size art used to depict the same card in game play is materially less like a search thumbnail.

### 4.2 Functional card/rules data

**Purpose and character — favors the user, with limits.** A private deterministic rules engine uses data to calculate legal actions, test rule coverage, reproduce results, and evaluate decks. That analytic/functional purpose differs from browsing collectible card content. It is noncommercial. The argument is strongest where exact text is retained only as source evidence and executable behavior is independently encoded.

**Nature — mixed, generally favorable for mechanics.** Rules, numeric parameters, identifiers, and compatibility fields are functional or fact-like. Flavor text, characters, and expressive prose point the other way.

**Amount — mixed.** Comprehensive simulation can reasonably require every implemented card’s functional fields and exact official text for verification. *Sega*, *Connectix*, and *Google* support taking what is necessary to reach/use functional material. Copying unused fields, flavor, layout, or art weakens necessity. A “complete coverage” research goal helps explain a full functional dataset, but does not make every field necessary.

**Market — currently favorable but uncertain.** One offline copy is not distributed, does not host an API substitute, and is unlikely to displace card sales or API traffic. However, courts consider the effect of widespread similar conduct and traditional or likely licensing markets. A publisher could plausibly license official digital games, data feeds, or simulator assets. Commercial launch or shared snapshots would sharply worsen this factor.

**Inference:** private functional use has a credible, fact-dependent fair-use argument, especially for mechanics and necessary intermediate source material. It is not a safe harbor, and reliance on it is unnecessary where the publisher confirms an API/cache license.

### 4.3 Artwork

**Purpose and character — weak to mixed.** Showing full card art during game play is visually useful but generally uses the art to depict the card, close to its original purpose. Local-only and noncommercial facts help. A small identification thumbnail may be somewhat more defensible than full-resolution art, but *Perfect 10* involved search indexing, not ordinary game display.

**Nature — strongly favors the owner.** Sorcery promotes its cards as hand-painted art. Pictorial works sit at copyright’s creative core.

**Amount — favors the owner.** A card image or art crop usually copies the entire image and its visual “heart.” The engine can function without it.

**Market — mixed to unfavorable.** One private copy presents little measurable substitution, but licensed digital games, art products, image APIs, and full-resolution assets are plausible markets. A public or commercial interface would be substantially riskier.

**Inference:** fair use for a complete local art library is meaningfully weaker than for rules data. Keep art opt-in, user-supplied, private, and technically separable; written permission is the clean path.

### 4.4 Effect of being private and noncommercial

Those constraints matter:

- no public distribution right is exercised;
- a one-person local display is not ordinarily a public display;
- fair-use factor one is more favorable;
- actual and potential market harm is smaller;
- ordinary civil enforcement is less likely as a practical matter;
- willful criminal copyright provisions generally target specified commercial, high-value reproduction/distribution, or prerelease public-distribution conduct. [17 U.S.C. § 506](https://www.copyright.gov/title17/92chap5.html).

They do **not** eliminate:

- the reproduction made by a download, scan, or cache;
- an enforceable contractual promise against automation or systematic retrieval;
- DMCA liability for circumventing an access control;
- CFAA/state risk if the user crosses an authorization gate or defeats blocks;
- civil injunction, damages, impoundment/destruction, and fee exposure if infringement is proved. [17 U.S.C. §§ 501–505](https://www.copyright.gov/title17/92chap5.html).

## 5. Website/API Terms, assent, and the API conflict

### 5.1 Formation

**Law (binding Ninth Circuit, applying California/New York principles):** a bare Terms hyperlink plus passive browsing usually does not establish constructive notice, but actual knowledge or affirmative assent can. *Nguyen v. Barnes & Noble Inc.*, 763 F.3d 1171, 1175–79 (9th Cir. 2014), [opinion](https://law.justia.com/cases/federal/appellate-courts/ca9/12-56628/12-56628-2014-08-18.html).

**Law (binding Ninth Circuit, 2024):** conspicuous notice paired with an action that unambiguously manifests assent can form an enforceable sign-in-wrap agreement. *Keebaugh v. Warner Bros. Entertainment Inc.*, 100 F.4th 1005, 1013–22 (9th Cir. 2024), [opinion](https://law.justia.com/cases/federal/appellate-courts/ca9/22-55982/22-55982-2024-04-26.html).

**Law (District of Alaska, predictive rather than binding Alaska Supreme Court authority):** Alaska appellate courts had not addressed clickwrap; a federal court applying Alaska law enforced terms the user had to accept before completing online check-in. *Werner v. Holland America Line, Inc.*, No. 1:18-cv-00018-TMB, 2019 WL 4540125, at *6–8 (D. Alaska Mar. 21, 2019), [order](https://www.govinfo.gov/content/pkg/USCOURTS-akd-1_18-cv-00018/pdf/USCOURTS-akd-1_18-cv-00018-0.pdf).

**Inference:** an anonymous first visit to an endpoint with no visible Terms notice presents a real formation issue. But this user now has actual knowledge of the Terms. Continued use after actual notice materially weakens any browsewrap defense. Any account creation, click-through, or sign-in can provide stronger assent evidence.

**Contract text:** the Terms choose New Zealand law. Alaska courts generally respect contractual forum/choice clauses subject to formation and strong-public-policy limits. See *ResQSoft, Inc. v. Protech Solutions, Inc.*, 488 P.3d 979, 983–87 (Alaska 2021), [opinion](https://law.justia.com/cases/alaska/supreme-court/2021/s-17548.html). **Uncertainty:** the public Terms’ arbitration mechanics and New Zealand-law enforceability require New Zealand-specific advice; U.S. copyright liability remains a separate federal question for copying occurring in Alaska.

### 5.2 Contract can matter even when data are public

**Law/instructive result:** the Ninth Circuit’s public-web CFAA decision did not immunize scraping from contract. On later summary-judgment proceedings, the district court found that hiQ’s agreed User Agreement prohibited its scraping and use, while factual waiver/estoppel issues remained for some conduct. *hiQ Labs, Inc. v. LinkedIn Corp.*, 639 F. Supp. 3d 944, 953–66 (N.D. Cal. 2022), [decision](https://caselaw.findlaw.com/court/us-dis-crt-n-d-cal/2182242.html). The matter ended by consent, so the settlement is not precedent.

**Law (binding Ninth Circuit):** violating a software-use covenant does not automatically become copyright infringement; the license condition must have a nexus to a § 106 right. *MDY Industries, LLC v. Blizzard Entertainment, Inc.*, 629 F.3d 928, 939–42 (9th Cir. 2010), [opinion](https://law.justia.com/cases/federal/appellate-courts/ca9/09-15932/09-15932-2011-02-25.html). It can still be a contract breach.

**Inference:** even if many individual API fields are unprotected, an enforceable promise not to automate or compile can create separate contract exposure. The best defense is permission, not an argument that copyright would allow the data use.

### 5.3 Manual versus automated acquisition

| Method | Contract/access assessment |
|---|---|
| Normal manual browser view, no durable bulk collection | Closest to the offered use and “standard browser” exception. Lowest contract risk. |
| Manual save of one rulebook or a few needed files | Supported by the personal/noncommercial download license if properly accessed; copying the entire art/database corpus may exceed the ambiguous “portion” language. |
| Manual export/save of the complete API response | Not “automated,” but may still be systematic retrieval to create a database. The API self-hosting guidance helps; written clarification remains advisable. |
| User personally launches a PowerShell script | It remains access through a script and automated means. Personal initiation does not change that classification. |
| An AI agent or browser-driving tool clicks/downloads | Also automated/non-human access. A human supervising each step does not necessarily turn the agent into ordinary manual browser use. |
| A public, documented, unauthenticated API call | Better CFAA posture and stronger implied-purpose evidence than scraping HTML; still subject to the unresolved Terms/robots conflict and rate limit. |
| Access after a block, 403, 429, revocation, or cease-and-desist | Stop. Switching IPs, credentials, endpoints, or agents to continue is materially higher risk. |

## 6. CFAA, automated access, and technical boundaries

**Law (federal statute):** the CFAA prohibits specified intentional access “without authorization” or exceeding authorized access to obtain information from a protected computer, fraud, or damage, and permits a civil action only for enumerated harm conditions. [18 U.S.C. § 1030](https://uscode.house.gov/view.xhtml?preview=true&req=%28title%3A18+section%3A1030+edition%3Aprelim%29&site_id=756).

**Law (U.S. Supreme Court):** “exceeds authorized access” is a gates-up-or-down inquiry about information the user is not entitled to obtain, not a broad prohibition on misuse of information the user may access. *Van Buren v. United States*, 593 U.S. 374, 389–96 (2021), [opinion](https://www.supremecourt.gov/opinions/20pdf/19-783_k53l.pdf).

**Law (binding Ninth Circuit):** publicly available webpages that anyone can reach with a browser have no authorization gate for CFAA purposes; a cease-and-desist alone did not convert those public pages into a gated system. *hiQ Labs, Inc. v. LinkedIn Corp.*, 31 F.4th 1180, 1195–1201 (9th Cir. 2022), [official opinion](https://cdn.ca9.uscourts.gov/datastore/opinions/2022/04/18/17-16783.pdf). The holding addressed the CFAA at the preliminary-injunction stage, not copyright, contract, trespass, or an entitlement to defeat authentication.

**Law (binding Ninth Circuit):** continuing to access password-protected data after individualized revocation and circumventing IP blocks was “without authorization.” *Facebook, Inc. v. Power Ventures, Inc.*, 844 F.3d 1058, 1067–69 (9th Cir. 2016), [official opinion](https://cdn.ca9.uscourts.gov/datastore/opinions/2016/07/12/13-17102.pdf).

**Executive-branch policy, not a defense:** DOJ’s 2022 CFAA charging policy says prosecutors should not bring “exceeds authorized access” cases based only on ordinary contract/Terms restrictions, with a narrow exception for terms that entirely bar access to particular resources. The policy is discretionary and creates no private right. [DOJ CFAA charging policy](https://www.justice.gov/d9/press-releases/attachments/2022/05/19/cfaa_policy_may_19_0.pdf).

Application:

- **Inference — lower CFAA risk:** one rate-compliant request to the public, unauthenticated `/api/cards` endpoint, using no evasion, looks like an ungated public resource under *Van Buren/hiQ*.
- **Inference — not zero:** the endpoint is rate-limited, and `/robots.txt` disallows bots. Neither fact necessarily creates a CFAA gate, but intentionally defeating the rate limiter, authentication, signed URLs, or a block would be a different case.
- **Inference:** a ToU violation alone is more naturally a contract issue after *Van Buren/hiQ*, but the same conduct can trigger other CFAA subsections if it causes damage, involves fraud, or crosses a technical gate.
- **Required stop conditions:** never rotate IPs or accounts to evade limits; never reuse someone else’s credential/token; never guess private endpoints; never bypass Drive permissions; stop on 401/403/429, CAPTCHA, an IP block, or publisher notice and seek permission.

## 7. DMCA § 1201 and technical controls

**Law:** 17 U.S.C. § 1201(a)(1) prohibits bypassing a technological measure that effectively controls access to a copyrighted work; § 1201(a)(2) and (b) restrict trafficking in circumvention technology. “Circumvent” includes decrypting or avoiding/bypassing the measure without the copyright owner’s authority. [Statutory text](https://www.copyright.gov/title17/92chap12.html).

**Law (binding Ninth Circuit):** § 1201(a) creates an access-control right that can exist independently of infringement of a traditional § 106 right. *MDY*, 629 F.3d at 944–52. *VidAngel* held that authorization to view a lawfully purchased disc did not itself authorize circumvention of its encryption. 869 F.3d at 863–67.

**Law:** § 1201(f)’s interoperability exception concerns lawfully obtained computer programs and necessary analysis of program elements, not a general license to decrypt card images or databases. Current triennial exemptions are narrow and class/use-specific; none identified on the Copyright Office’s [2024 final-rule page](https://www.copyright.gov/1201/2024/) is an obvious blanket exemption for a private TCG art/data library.

**Inference:** ordinary requests to a public JSON endpoint and ordinary access to a public Drive folder do not appear to require circumvention. Rate limiting alone should be obeyed, not tested. If a private CDN requires signed URLs, authentication, encryption, obfuscated tokens, or another access process, do not bypass it—even if a URL pattern can be inferred and even if the resulting local use might otherwise be fair.

**Uncertainty:** `robots.txt` is a machine-readable preference, not encryption, and ordinarily does not itself “effectively control access” because no authorized information/process is required to load the work. That does not make ignoring it contractually acceptable here, particularly given the express automation ban.

## 8. Alaska law and other jurisdictions

### 8.1 Alaska computer law

**Law:** Alaska’s current criminal-use-of-computer statute requires knowing unauthorized/excess access plus one of listed results, including obtaining information about a person, nonpublic “proprietary information,” fee-only information, damaging/tampering conduct, or encryption/decryption. “Proprietary information” expressly means covered information the holder has not made available to the public. [AS 11.46.740, official Title 11 PDF](https://www.akleg.gov/statutesPDF/Title-11.pdf).

**Inference:** public, free card data and a public image folder do not fit the statute’s most relevant nonpublic/fee-only categories on the stated facts. The risk changes if access restrictions, private CDN content, tokens, encryption, another person’s account, or damaging traffic are involved. There is little reported Alaska authority applying this section to ordinary public-web scraping, so avoid testing its edges.

**Uncertainty:** no materially on-point Alaska Supreme Court decision or generally applicable Alaska civil computer-trespass statute was located for a harmless request to public game data. Contract, common-law interference/trespass theories, and federal law can still apply. Server-overload or post-block evasion would create facts absent from this project.

### 8.2 Database rights

**Law — United States:** the United States has no general EU-style sui generis database right. Copyright protects original database selection/coordination/arrangement and any protected content, not raw facts or collection effort. *Feist*, 499 U.S. at 348–64; [Copyright Office database guidance](https://www.copyright.gov/register/tx-databases.html). Contract, trade secret (for nonpublic information), and narrow state tort theories can operate separately.

**Law — European Union:** Directive 96/9/EC provides a separate right against extraction or reutilization of a substantial part of a qualifying database, and repeated/systematic extraction of insubstantial parts in specified circumstances. [Directive 96/9/EC, consolidated text](https://eur-lex.europa.eu/legal-content/EN/TXT/PDF/?uri=CELEX%3A01996L0009-20190606). Eligibility, location of acts, and territorial application require separate analysis.

**Inference:** an Alaska-only private copy is principally a U.S. question, plus any enforceable New Zealand contract. A public service offered in the EU/UK or acquisition performed there needs a new database-right review.

### 8.3 New Zealand overlay

The Terms select New Zealand law. New Zealand’s Copyright Act has a fair-dealing provision for research or private study that considers purpose, nature, reasonable commercial availability, market effect, and amount, and generally limits the provision to one copy on an occasion. [Copyright Act 1994 § 43, version as at Nov. 13, 2025](https://legislation.govt.nz/act/public/1994/143/en/2025-11-13.pdf). Its TPM rules differ from U.S. § 1201. [Copyright Act 1994 §§ 226–226E](https://www.legislation.govt.nz/act/public/1994/0143/108.0/whole.html).

**Uncertainty:** choice-of-law clauses ordinarily govern contract, not automatically every territorial copyright issue. This report does not resolve New Zealand contract formation, fair dealing, jurisdiction, or arbitration. Those issues are a reason to seek written permission rather than rely on a U.S.-only fair-use analysis.

## 9. User-run tool versus bundled content

**Law:** the person whose script downloads a file generally makes the direct copy. A tool distributor can face secondary copyright liability if it affirmatively promotes infringement; mere knowledge of possible misuse is not the same as inducement. *Metro-Goldwyn-Mayer Studios Inc. v. Grokster, Ltd.*, 545 U.S. 913, 928–40 (2005), [official U.S. Reports PDF](https://tile.loc.gov/storage-services/service/ll/usrep/usrep545/usrep545913/usrep545913.pdf).

**Inference:** making the owner run the script avoids bundling and redistribution by the repository maintainer, and it can preserve many lawful uses. It does not authorize the owner’s request, erase direct copying, or eliminate a claim if the tool is designed/marketed to defeat restrictions.

A safer tool design:

- contains no official JSON, rules PDFs, card text, or artwork;
- accepts a local path and works fully offline after import;
- makes art optional and user-supplied, with no hidden default downloader;
- if written API permission is obtained, sends a clearly identified, slow, single-source request; honors `ETag`, caching, rate headers, retries, and stop conditions;
- never falls back from the public API/Drive to HTML scraping or a private CDN;
- records source URL, retrieval time, source version/hash, and permission basis privately;
- keeps all imported content ignored, outside packages/releases/tests, and out of telemetry;
- makes no claim of affiliation, endorsement, or official status;
- requires a new permission/legal review before enabling sharing, hosting, CI downloads, model-provider uploads, or commercial use.

## 10. Relative risk matrix

“Lower” is relative, not “cleared” or guaranteed lawful.

| Conduct under the stated Alaska/private/noncommercial facts | Relative risk | Why / controlling boundary |
|---|---:|---|
| Read the official rulebook/card library manually in a normal browser | Lower | Offered ordinary access; no durable bulk library beyond ordinary browser operation. |
| Manually download the official rulebook for private reference | Lower | Officially offered file plus Terms’ personal/noncommercial portion license; keep it private. |
| Implement mechanics in original code, with original labels/explanations | Lower | Methods/systems are outside copyright; avoid copied prose, art, and marks. |
| Import a user-created deck list containing names and quantities | Lower | Predominantly short identifiers/functional facts; no art or official database bundled. |
| Use a user-supplied, lawfully obtained local data file; no network access | Lower | Tool does not retrieve or redistribute publisher content; source/user authorization still matters. |
| Clone/use SadKingLabs MIT code with notice, but independently source official data | Lower | Clear code license; maintains content-license separation. |
| Private display of already authorized local images to the owner only | Lower–Medium | Not a public display; the source/reproduction authorization remains the issue. |
| Manually save the complete public API JSON once for local use | Medium | API self-hosting guidance helps; full systematic collection and Terms scope/assent remain unresolved. |
| User runs a one-shot or intermittent API script, rate-compliant and cache-aware | Medium | Exactly what API guidance appears to contemplate, but expressly conflicts with Terms and robots directive. Written permission would move this toward Lower. |
| Agent/browser automation performs the same download | Medium | Still automated/non-human; supervision does not cure the Terms conflict. |
| Vendor SadKingLabs `registry.json` privately | Medium | No art and strong functional use, but MIT is expressly limited to code and the export contains Erik content. |
| Manually download only the images needed for one owned deck from the linked public folder, local display only | Medium | Public folder and personal use help; whole creative images and license scope remain uncertain. |
| Scan owned physical cards into local image files | Medium | Avoids website contract/access issues but creates new reproductions; card ownership does not grant a copying right. |
| Download all released artwork by script, even only once and privately | Higher | Entire highly creative corpus; weak necessity; automation/systematic retrieval; no express bulk-image license. |
| Infer/use private-CDN URLs, hotlink, proxy, or serve CDN images | Higher | Express publisher prohibition; potential contract, CFAA, and DMCA issues depending on controls. |
| Bypass 429/403/CAPTCHA/authentication, rotate IPs/accounts, or continue after notice | Higher | Crosses from public access toward *Power Ventures* and possible access-control circumvention. |
| Commit or release official API snapshots, rulebook PDFs, card text corpus, or SadKingLabs data export | Higher | Adds distribution, creates an API/database substitute, and exceeds the private-use premise. |
| Commit, host, or publicly display card images/art | Higher | Reproduction/distribution/public display of highly creative works; possible trademark/endorsement issues. |
| Upload source materials/art to a model provider or third-party service | Higher | Disclosure/transmission to another party; provider retention/training terms add another license issue. |
| Public multiplayer/hosted simulator or commercial release using official content | Higher | Commercial and public uses, potential digital-game/licensing market substitution, Terms prohibition. Obtain a negotiated license first. |

## 11. Permission questions

Send Erik’s Curiosa a single, concrete written request at `community@sorcerytcg.com` and retain the response. Ask for explicit answers to each item rather than a general “fan project okay?”

1. Does the public API page authorize an Alaska individual to run a script against `/api/cards` despite the January 15, 2024 Terms’ automation/systematic-retrieval clauses and the API host’s `robots.txt`?
2. What polling frequency, rate/concurrency, identification/User-Agent, `ETag` behavior, and retry rules are permitted?
3. May the user keep complete timestamped/hash-pinned API snapshots indefinitely for deterministic replay, backups, and audit?
4. May the user normalize fields, add stable local IDs, compute hashes/derived statistics, and encode mechanics for a private simulator?
5. May exact card rules text and official rulings be retained and displayed locally? Does permission include historical/errata versions?
6. Which fields must be excluded (especially flavor text, lore, unpublished data, artist metadata, or promotional content)?
7. May the user download images from the linked public Drive? If yes: all released cards or only owned/needed cards; manual or scripted; what resolution; one local copy plus backup; crops/thumbnails; in-simulator display?
8. Does Erik’s Curiosa have authority from the individual artists/rightsholders to grant that image permission, and are attribution/copyright notices required?
9. Confirm that private-CDN access, hotlinking, proxying, serving, token reuse, and URL inference are excluded.
10. May the project publish source code with **no** official data/art, while each user separately supplies or fetches content? What wording/disclaimer and marks may the repository use?
11. May tests contain minimal card names, numeric fields, or short rules excerpts, or must all fixtures be synthetic?
12. Does permission cover private backups and migration to another personally controlled machine? Does it cover an offline local browser GUI?
13. What happens on permission revocation: stop future polls only, or delete retained snapshots/images? How will changes be notified?
14. Is any public sharing, hosted play, research publication, benchmark output, or commercial use permitted? Treat each as **no** unless separately written.

Ask SadKingLabs separately:

1. Does MIT cover only executable/source code, or also the JSON Schema, stable ID assignments, slug-history structure, checksums, and SadKing-authored corrections?
2. Under what license may those SadKing contributions be copied, modified, vendored, and redistributed without Erik’s card fields?
3. What attribution/notice is required for generated identifiers and transformed exports?
4. Can the simulator run the MIT pipeline against its own authorized private API snapshot without vendoring SadKing’s republished card data?

SadKingLabs cannot answer Erik’s Curiosa content/art permission questions.

## 12. Conservative operating boundary

Until written clarification arrives:

1. Ship engine code only. No official card/rules files, SadKing data export, art, or source-derived fixtures in Git, packages, releases, CI artifacts, logs, or model prompts.
2. Use manual browser acquisition for the small official authority set the user needs immediately, under the Terms’ personal/noncommercial license. Keep it ignored, access-controlled, and local.
3. Do not deploy the PowerShell/API collector. The public API’s technical invitation is real, but the Terms and `robots.txt` conflict is now known.
4. Implement mechanics and state transitions in original language. Preserve source citations/hashes privately; do not copy rulebook prose into code comments or docs beyond short necessary references.
5. Exclude flavor/lore and artwork. For UI development, use synthetic placeholders.
6. If private art is essential before permission, use only user-supplied, deck-scoped files from an ordinary manual download; do not bulk-download, scan the full collection, access the private CDN, or backfill automatically. This remains a medium-risk compromise, not a clearance.
7. Stop immediately upon any technical block or publisher objection. Do not evade it.
8. Re-review before any public repository feature that automatically fetches content, public benchmark containing card text, hosted GUI, model-provider upload, redistribution, or monetization.

Once Erik’s Curiosa gives sufficiently specific written permission, the minimal collector should perform a single cache-aware request, retain only required fields, honor the stated limit and `ETag`, keep data private/ignored, and require a deliberate separate opt-in for any separately licensed artwork.

## 13. Authority hierarchy and limitations

Most important binding authorities for an Alaska user are the U.S. Supreme Court and Ninth Circuit cases linked above. District-court scraping and online-contract rulings are persuasive, not binding. Copyright Office pages are authoritative agency guidance and statutory reproductions, not judicial holdings. The publisher/API/README pages establish current public statements and license text, not their ultimate enforceability or ownership chain.

No search can establish that Erik’s Curiosa owns every relevant art right, that every work is registered, or that no separate artist agreement applies. No definitive Sorcery-specific reported decision was located. The current official pages can change without notice; preserve a dated PDF/HTML capture and hash of the Terms, API guidance, `robots.txt`, permission correspondence, and every source snapshot used.
