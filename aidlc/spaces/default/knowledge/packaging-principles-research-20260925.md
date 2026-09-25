# 良いパッケージングの条件 — 文献調査（2026-09-25）

コードをモジュール・パッケージ・クレートへ切り分けるときの「良い条件」を、文献で確かめた記録である。出発点は、オーナーの仮説「低結合・高凝集」「チャンクアップ／チャンクダウン（抽象度の階層化・認知負荷）」「MECE」の 3 つだった。

この文書は規則ではない。規則の正本は [coding-rules](aidlc-shared/coding-rules/README.md) で、ここはその根拠として参照する調査記録である。

**調査の方法と制約**: 検索は arXiv API・OpenAlex API（被引用の追跡）・Web 検索で行い、本文は `paper` CLI・Web 取得・`pdftotext` で読んだ。`paper-search` の Google 検索とページ取得は API キー未設定で使えず、Semantic Scholar はレート制限（429）で使えなかった。参考文献の節で、原文を読んだもの・要旨や二次資料だけのもの・記憶によるものを分けて示す。

## 1. 結論

- **低結合・高凝集: 条件付きで支持。** 何に対する結合・凝集かを決めないと中身がない。文献は「一緒に変わる理由」（Parnas の情報隠蔽、Martin の CCP）と「概念」（Evans）で測るよう求めている。構造上の依存を数えるだけのメトリクスでは足りない。
- **チャンクアップ・ダウン: 条件付きで支持。** 作業記憶は約 4 チャンクと小さく、分割が必要な根拠になる。ただし実験では、細かく分けすぎると理解が遅くなった。また Parnas は「階層があること」と「きれいな分割であること」を独立した性質としている。
- **MECE: 不十分。** 「ひとつの設計判断は一つのモジュールだけが隠す」という排他性（ME）は有効である。しかし網羅性（CE）は分割の基準にならない。横断的関心事や原則同士の緊張関係のために、正しい分割がひとつに定まらないからである。
- **仮説に足すべき条件**: (a) 変わる理由（変更軸）で切る、(b) 依存の向き・非循環・安定度、(c) インタフェースが深いこと（情報漏洩がないこと）、(d) 結合の強さと近さ（connascence）、(e) ドメインの語彙と組織の境界への対応、(f) 再編コストへの配慮。

## 2. 良いパッケージングの条件

| # | 条件 | 中身 | 根拠 | 測り方・検出 |
|---|---|---|---|---|
| C1 | 変わりやすい設計判断を隠す | 処理の順序（フローチャート）ではなく、変わりやすい判断ごとにモジュールを切る。インタフェースは内部を最小限しか明かさない | Parnas 1972。結論節に「変わりそうな設計判断の一覧から始める」とある | 自動検出は難しい。co-change（一緒に変更される頻度）で事後に検証する |
| C2 | 同じ理由で変わるものを集める | 一つの変更は一つのパッケージで閉じるのが理想 | Martin 1996（CCP） | co-change 分析。Clio、PairSmell |
| C3 | 一緒に使うものを集める | 使わない型の変更に巻き込まれないようにする | Martin 1996（CRP）、Rust API Guidelines | 利用側がパッケージ内の型をどれだけ使っているかの比率 |
| C4 | 依存に循環がない | パッケージ間の依存グラフが DAG になっている | Martin 1996（ADP）。Parnas も、下位が上位に依存しない階層なら上位を切り落とせると述べる | 機械的に検査できる（循環検出、DSM） |
| C5 | 安定しているものに依存する | 変わりにくいものほど抽象的にする | Martin 1994 の指標: I = Ce/(Ca+Ce)、A = 抽象型の比率、D = \|A+I−1\|/2。Rust の C-STABLE は同じ考え方 | I・A・D は計算できる |
| C6 | モジュールが深い | 小さいインタフェースで多くの機能を提供する。処理順で分ける分割（temporal decomposition）と情報漏洩を避ける | Ousterhout（二次資料で確認） | 公開 API の数と実装量の比率。薄く中継するだけの型の検出 |
| C7 | 強い結合は近くに置く | 結合を強さ・広がり・近さの三軸で評価する | connascence.io（源流は Page-Jones — 記憶） | 名前や型の結合は静的に検出できる。動的な結合はほぼ検出できない |
| C8 | ドメインの物語を語る | モジュール名をユビキタス言語の一部にする | Evans『DDD Reference』の Modules | 用語集と名前の照合（半自動） |
| C9 | 組織・所有の境界と合う | 設計は組織のコミュニケーション構造を写す | Conway 1968、Team Topologies | 実証の結果は割れている（§4） |
| C10 | 再編コストを考える | 最適化すると構造の 57% が変わる。開発者はその混乱を避ける | Paixao ら 2017 | MoJoFM（再編の規模） |

## 3. 仮説の検証

### 低結合・高凝集

- **支持する証拠**
  - Stevens・Myers・Constantine（1974）は、「部品を、他への影響を最小にして考え・直せるように分ける」ことを簡潔さの基準にした（抜粋のみ取得）。
  - Clio（Wong ら 2011）は、「同じ理由で変わるはずなのに別モジュールにある」違反を検出した。Hadoop では検出した 231 件のうち 152 件（65%）が、後に設計上の問題と認められたかリファクタされた。
  - PairSmell（Zhong ら 2024、20 プロジェクト）では、分け方が不適切なペアは適切なペアより co-change が 190% 多かった。
  - Hotspot（Mo ら 2015）も、構造と変更履歴を併せたパターンがバグと変更の多さに最も効くと報告している。
- **限界**
  - 構造だけのメトリクスは一つの次元しか見ない。
  - Homay（2025、ポジションペーパー）は、「高い・低い」の閾値に根拠がないと批判している。
  - Paixao らによれば、開発者の設計は凝集・結合の点で無作為な分割より有意に良いが、改善の余地が平均 25% 残っていた。
- **緊張関係**
  - REP と CCP はパッケージを大きくし、CRP は小さくする。Martin は三角形の緊張図（tension diagram）で、この釣り合いは開発が進むにつれて動くと述べている（二次資料で確認）。
  - 再利用のために共通化する（DRY）と、共通部品を使う側すべてに結合が生まれる。connascence の言葉では「広がり」が増える。

### チャンクアップ・ダウン

- **支持する証拠**
  - Cowan（2001）は、作業記憶を 3〜5 チャンクとした。Miller の 7±2 は目安にすぎない。
  - Evans は、低結合と高凝集をそれぞれ認知の限界から説明している。「一度に考えられる数には限界がある（だから低結合）」「バラバラな断片は、スープのように区別のない塊と同じくらい理解しにくい（だから高凝集）」。
  - The Rust Book は、カプセル化を「頭に入れておく詳細の量を減らす方法」と説明している。
  - Siegmund らの fMRI 研究は、人がコードを意味のまとまり（チャンク）単位で理解していることを示した（要旨を確認）。
- **反例**
  - Costa ら（2026、初学者 32 名の視線計測）では、簡単な課題でメソッドを抽出すると時間が最大 167% 増えた。呼出元と抽出先を行き来する移動が増えるためである。
  - Segalotto ら（2023、脳波）では、モジュール化したコードは認知負荷が低い一方で時間は長く、正答率は上がらなかった。
  - つまり、分けすぎると浅いモジュールが増えて、かえって負荷になる（Ousterhout の classitis 批判と同じ方向）。
- Parnas は「階層構造」と「きれいな分割」を独立した性質としている。抽象度を階層にしても、それだけでは良い分割にならない。

### MECE

- **成り立つ部分**
  - 各設計判断を一つのモジュールだけが隠す、という意味での排他性。
  - Conway・Team Topologies の意味での所有の排他性。
- **崩れる部分**
  - Tarr ら（1999）の「支配的な分割の専制」。一度に一つの軸でしか分けられず、それ以外の関心事は複数のモジュールに散らばって絡み合う。
  - REP・CCP・CRP の緊張関係があるため、正しい分割がひとつに定まらない。
  - Martin 自身が、パッケージ構造は「ビルドの地図」で、設計の進行とともに変わるものであり、機能を分解したものではないと書いている。
  - DDD の Bounded Context は、同じ用語を境界ごとに別のモデルとして持つことを意図的に許している。
- 網羅性（CE）はテストの網羅には効くが、分割の良し悪しの基準にはならない。

## 4. 文献間の一致点と不一致点

- **一致する点**: 「変更を局所化する」ことが中心的な目的だという点は、Parnas、Martin、Evans、Clio、Hotspot、Baldwin & Clark（設計ルールと、見えるモジュール／隠れたモジュール）で共通している。
- **一致しない点**
  - 組織との整合（Conway）について、Colfer & Baldwin（2016）が 142 件の研究をレビューした結果、企業では広く見られるが普遍的ではなく、OSS ではほぼ支持されなかった。
  - Mauerer ら（2021）の大規模な縦断研究では、社会技術的な整合性はバグ・バグ密度・変更量と実質的な関係がなかった。Cataldo らの「修正依頼の解決時間が 32% 短縮」とは食い違う。
- **実証の強さと限界**
  - co-change 系の研究はどれも相関で、Java と C/C++ の OSS に偏っている。
  - Martin の I・A・D が欠陥を予測できるかを確かめた強い研究は見つからなかった。
- **文献の空白**
  - package by feature と package by layer を比較した厳密な実証研究は見つからなかった（ブログの主張が中心）。
  - Rust のモジュール・クレート分割の実証研究も見つからなかった。

## 5. amadeus-ng への示唆

### 既存の規則を補強するもの

- **公開面を絞った `pub use` ファサード**（[module-visibility.md](aidlc-shared/coding-rules/module-visibility.md)）: The Rust Book の「内部構造と、利用者がドメインを考える構造は違ってよい」がそのまま根拠になる。モジュールを既定で非公開にする方針も、Rust API Guidelines の C-STRUCT-PRIVATE / C-NEWTYPE-HIDE と一致する。
- **1 ファイル 1 公開型**（[abstract-data-type.md](aidlc-shared/coding-rules/abstract-data-type.md)）: 探しやすさの点で有効である。一方で、実験の結果からは浅い型が増えすぎる危険がある。その釣り合いを取るのは、チャンクの単位になるファサードの設計である。ファサードの設計を、この規則と対にして扱う。

### ドメイン層の技術駆動パッケージングの禁止（2026-09-25 オーナー裁定）

ドメイン層（`modules/core/command/domain/src`）は、`entities/`・`value_objects/` のようなパターンの種類で分けず、境界づけられたコンテキストと、型が所有するサブツリーで分ける。規則は [domain-packaging.md](aidlc-shared/coding-rules/domain-packaging.md) にある。本調査との対応は次のとおり。

- C2（同じ理由で変わるものを集める）: 1 つのドメイン概念への変更は、その集約・値オブジェクト・イベントを一緒に変える。種類で分けると 1 つの変更が複数のディレクトリに散らばり、co-change が境界をまたぐ。
- C8（ドメインの物語を語る）: `value_objects` はドメインの語ではなく実装の語である。Evans『Domain-Driven Design』の Modules の章は、これを技術駆動パッケージング（Infrastructure-Driven Packaging）の落とし穴として戒めている（原文は今回未取得 — 記憶による）。
- 「package by feature と by layer の厳密な比較研究は見つからなかった」（§4）のは実証の空白であって、種類別の分割を支持する証拠ではない。

### 最近の課題の診断

- **`ReadModelUpdater` 系に共通の trait 契約がなかった件**: Baldwin & Clark の「設計ルール」（見えるモジュール）が欠けていた状態であり、SAP（安定しているものほど抽象的に）にも反する。安定した中間クレート（`core-read-model-updater`）に trait を置き、具体的な実装がそれに依存する形にするのが理にかなう。
- **`JournalReader` がジャーナルの読みとリードモデルへの書きを兼ねている件（#153）**: CRP・ISP 違反であり、CQRS の境界をまたいだ情報漏洩である。名前が責務と合っていない点は、Evans の命名原則にも反する。

### lint や定期レポートにできそうなもの

1. クレートと、クレート内モジュールの依存方向の検査（ADP・SDP）。`cargo metadata` などを使い、コマンド側がリードモデル側・クエリ側に依存しないことを確かめる。
2. `*Reader` / `*Query` という名前のポート trait に、書き込み系のメソッド（`&mut self`、`save`・`write`・`apply` など）を置かない検査。
3. 同じ接尾辞の型の集まり（`*ReadModelUpdater` など）に、共通 trait の実装を義務付ける名前規則。
4. git 履歴の co-change から、Clio や PairSmell 相当のレポートを作る。Bounded Context をまたいで頻繁に一緒に変わるファイルの組は、分割の見直し候補になる。
5. クレートごとの I・A・D を計算する。欠陥の予測力は実証されていないので、参考値として扱う。

### 注意点

- Bounded Context 間の用語の重複は、MECE 違反ではなく、DDD として意図されたものである。
- Paixao らの結果（最適化は構造の 57% を変える）から、大規模な再編は一度に行わず、Bolt 単位で少しずつ進める。
- 組織との整合（Conway）は実証が割れているので、根拠の主軸にはしない。

## 6. 参考文献

### 本文を読んだもの

- Parnas, D. L. "On the Criteria To Be Used in Decomposing Systems into Modules." *Communications of the ACM* 15(12), 1972. https://wstomv.win.tue.nl/edu/2ip30/references/criteria_for_modularization.pdf
- Martin, R. C. "Granularity." *C++ Report*, 1996. https://condor.depaul.edu/dmumaugh/OOT/Design-Principles/granularity.pdf
- Martin, R. C. "OO Design Quality Metrics: An Analysis of Dependencies." 1994. https://linux.ime.usp.br/~joaomm/mac499/arquivos/referencias/oodmetrics.pdf
- Wong, S., Cai, Y., Kim, M., Dalton, M. "Detecting Software Modularity Violations." ICSE 2011. https://web.cs.ucla.edu/~miryung/Publications/icse11-modularityviolation.pdf
- Paixao, M., Harman, M., Zhang, Y., Yu, Y. "An Empirical Study of Cohesion and Coupling: Balancing Optimisation and Disruption." *IEEE TEVC*, 2017.（要旨と序論） https://discovery.ucl.ac.uk/1576532/1/Paixao_Empirical_study_cohesion.pdf
- Zhong, C. ほか "PairSmell: A Novel Perspective Inspecting Software Modular Structure." arXiv:2411.01012, 2024.（要旨と序論）
- Mauerer, W. ほか "In Search of Socio-Technical Congruence: A Large-Scale Longitudinal Study." arXiv:2105.08198, 2021.（結論）
- Evans, E. *Domain-Driven Design Reference*, 2015.（Modules の節） https://www.domainlanguage.com/wp-content/uploads/2016/05/DDD_Reference_2015-03.pdf
- *The Rust Programming Language* 第 7 章. https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
- *Rust API Guidelines*. https://rust-lang.github.io/api-guidelines/checklist.html

### 要旨・二次資料のみ

- MacCormack, A., Rusnak, J., Baldwin, C. *Management Science*, 2006. https://doi.org/10.1287/mnsc.1060.0552
- Mo, R. ほか "Hotspot Patterns." WICSA 2015. https://doi.org/10.1109/wicsa.2015.12
- Mo, R. ほか "Decoupling Level." ICSE 2016.
- D'Ambros, M., Lanza, M., Robbes, R. WCRE 2009. https://doi.org/10.1109/wcre.2009.19（変更の結合と欠陥の相関という結果は記憶による）
- Colfer, L., Baldwin, C. "The mirroring hypothesis." 2016. https://doi.org/10.1093/icc/dtw027
- Segalotto ほか 2023. https://doi.org/10.1145/3613372.3613387
- Costa ほか. arXiv:2602.18579（JSS 2026）
- Homay. arXiv:2507.09596
- Cowan, N. 2001.（*BBS* 24(1)）
- Sweller, J. 1988.（*Cognitive Science* 12(2)）
- Siegmund, J. ほか. ICSE 2014 / FSE 2017.
- Tarr, P. ほか. ICSE 1999.
- Conway, M. 1968. https://www.melconway.com/Home/Committees_Paper.html
- Baldwin, C., Clark, K. *Design Rules*（SSRN 312404）
- connascence.io
- Ousterhout『A Philosophy of Software Design』、Team Topologies、Clean Architecture の緊張図、Minto の MECE（いずれも要約記事）

### 記憶のみ（原文を取得していない）

- Stevens ほか 1974 の凝集度の分類の詳細
- Miller 1956
- Page-Jones 1992、Weirich

### BibTeX（主要文献）

```bibtex
@article{parnas1972criteria, author={Parnas, David L.}, title={On the Criteria To Be Used in Decomposing Systems into Modules}, journal={Communications of the ACM}, volume={15}, number={12}, pages={1053--1058}, year={1972}}
@article{martin1996granularity, author={Martin, Robert C.}, title={Granularity}, journal={C++ Report}, volume={8}, number={10}, pages={57--62}, year={1996}}
@inproceedings{wong2011clio, author={Wong, Sunny and Cai, Yuanfang and Kim, Miryung and Dalton, Michael}, title={Detecting Software Modularity Violations}, booktitle={Proc. ICSE}, pages={151--160}, year={2011}}
@article{zhong2024pairsmell, title={PairSmell: A Novel Perspective Inspecting Software Modular Structure}, author={Zhong, Chenxing and Feitosa, Daniel and Avgeriou, Paris and Huang, Huang and Li, Yue and Zhang, He}, journal={arXiv preprint arXiv:2411.01012}, year={2024}}
@article{mauerer2021stc, title={In Search of Socio-Technical Congruence: A Large-Scale Longitudinal Study}, author={Mauerer, Wolfgang and Joblin, Mitchell and Tamburri, Damian A. and Paradis, Carlos and Kazman, Rick and Apel, Sven}, journal={arXiv preprint arXiv:2105.08198}, year={2021}}
```

最後の 2 件は `paper bibtex` で生成したものに著者と年を補った。残りは手書きである。
