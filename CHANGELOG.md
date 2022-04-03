# Yary Changelog

## [0.1.2 - 2022-04-03](https://github.com/dolysis/yary/releases/tag/0.1.2)

[v0.1.1..0.1.2](https://github.com/dolysis/yary/compare/ee8c93eb3ded564c0bd94597e3a8855535610978..fcf85c98bd605846ef2808321d8e87259bc5f003)
### Build

- Refuse to bump-version when branch is not master ([fcf85c9](https://github.com/dolysis/yary/commit/fcf85c98bd605846ef2808321d8e87259bc5f003))
- Ensure tag is prefixed with a 'v' in git ([5c94b58](https://github.com/dolysis/yary/commit/5c94b5802b05b47602a69c423326f244629203be))
- Add changelog & patchnotes recipes, modify bump-version ([e6351d1](https://github.com/dolysis/yary/commit/e6351d1f89d082ba1f8877d8a62f63b4bd2bc481))
- Add simple patchnotes template for git tags ([bafd009](https://github.com/dolysis/yary/commit/bafd0098bdd4e53a79c450d0af685557ff916c7a))
- Add git-cliff configuration ([a2fab8d](https://github.com/dolysis/yary/commit/a2fab8db4f4d39c3f58b33bc4899fbfae08e9f09))
- Add recipes for just ([b53e7e7](https://github.com/dolysis/yary/commit/b53e7e78a950f598362878aef2207ba0daae8ade))

### CI/CD

- Add lint workflow ([6025906](https://github.com/dolysis/yary/commit/602590671983baaa16fd881ebfdd10a826561675)) <sup>ci</sup>
- Refactor tests workflow to use just ([7d4e270](https://github.com/dolysis/yary/commit/7d4e270aaef3a4a59d9af67e4fdcdde0ba38f642)) <sup>ci</sup>

### Chores

- Update lock file ([7d74a26](https://github.com/dolysis/yary/commit/7d74a265073878a702d6534cf7769bd51f666ad8))

### Lib.Event

- Fix module doc intra links ([afa2a9e](https://github.com/dolysis/yary/commit/afa2a9e5047a922c064e9d3c1176e112a0e1dc1a))

## [v0.1.1 - 2022-03-27](https://github.com/dolysis/yary/releases/tag/v0.1.1)

[v0.0.0..v0.1.1](https://github.com/dolysis/yary/compare/847cf54400c3f815bdba4cf9fa2016c65f00c273..ee8c93eb3ded564c0bd94597e3a8855535610978)
### Build

- Add pre-publish fields to Cargo.toml ([fa821cb](https://github.com/dolysis/yary/commit/fa821cbf8195a4d87d5aa115ed3ae2e3847ce6ab))

### CI/CD

- Add MSRV == 1.52 ([d3fd96e](https://github.com/dolysis/yary/commit/d3fd96ea310893dc63e46a7391d67cb57032bc9b)) <sup>ci</sup>
- Improve test naming ([f288b71](https://github.com/dolysis/yary/commit/f288b71f832fe259821998d0f9404ae041c80bcb)) <sup>ci</sup>
- Improve toolchain install task ([c14ba78](https://github.com/dolysis/yary/commit/c14ba7829c1d6457e3b32dfa94b5bb3b1a278956)) <sup>ci</sup>
- Add matrix targets for test_lazy ([7d90804](https://github.com/dolysis/yary/commit/7d90804cc53b08db48fbbd0d64db15bfef545bf4)) <sup>ci</sup>
- Add matrix test for feature.test_buffer_small ([98f6eb9](https://github.com/dolysis/yary/commit/98f6eb9a86832bb1793c4012c0939c3304a12312)) <sup>ci</sup>
- Fix error, allow styles ([0de27e1](https://github.com/dolysis/yary/commit/0de27e10925a99ab8f46a9552f2f9ae0c8170ed8)) <sup>ci</sup>
- Move rustfmt.toml to repo root ([e7882ed](https://github.com/dolysis/yary/commit/e7882ed59908462a5ee2fb29e20dcd03fb05e38c)) <sup>ci</sup>
- Use install over mv ([bee5588](https://github.com/dolysis/yary/commit/bee55887f0159de5237c967f7099d66516a3a2a6)) <sup>ci</sup>
- Fix sccache version, improve curl output ([7594379](https://github.com/dolysis/yary/commit/759437948823b1eccb062bcb3cb472fd4b654173)) <sup>ci</sup>
- Move actions to repo root ([d4e13dd](https://github.com/dolysis/yary/commit/d4e13dd1679e613b31041ef20a1eb6d412cfa6c8)) <sup>ci</sup>
- Add actions for PR unit testing ([5cdf7bf](https://github.com/dolysis/yary/commit/5cdf7bfc10c405a6a2f7fa99057a591626cfb3df)) <sup>ci</sup>

### Cargo

- Add feature.test_lazy ([8d01532](https://github.com/dolysis/yary/commit/8d01532b1f23bab57ecc7ec6057c7d3c91869c51)) <sup>tests</sup>
- Dev-dependencies.cfg-if = 1 ([70fea61](https://github.com/dolysis/yary/commit/70fea619cb0ec7d66e64874355be63dcaa04a2e5))
- Add dependencies.bitflags = 1 ([6147424](https://github.com/dolysis/yary/commit/6147424c3086c9c5ccf31ff76f6ce01d234a934d))
- Dependencies.anyhow -> dev-dependencies.anyhow ([c1aeb0d](https://github.com/dolysis/yary/commit/c1aeb0d3f0daeb9acff7b7eb5e27b4039d45cf0f))

### Chores

- Bump v0.1.1 ([ee8c93e](https://github.com/dolysis/yary/commit/ee8c93eb3ded564c0bd94597e3a8855535610978))
- Update deps ([44d8b82](https://github.com/dolysis/yary/commit/44d8b821295d85333e84e61403e7c564dde71bca))

### Documentation

- Fix rustdoc links in lib/scanner & lib/token ([8ee7b7b](https://github.com/dolysis/yary/commit/8ee7b7bff644557c1bb843984310b65ee120606b))
- Add README, explaining library purpose and status ([790b9b5](https://github.com/dolysis/yary/commit/790b9b55d50d604722d6faa8e3317767c7c49348))
- Commit notes ([a0d71ef](https://github.com/dolysis/yary/commit/a0d71ef644c7ab1be8b8daaa3ed1f45cb51f6c57))
- Notes from scalar line joining ([82d2719](https://github.com/dolysis/yary/commit/82d27199b5b367391627c2ca1b6fcc6e6bca1cbf))

### Lib

- Warn on missing public docs ([74cf9a2](https://github.com/dolysis/yary/commit/74cf9a2ad4c0b4435e24c8b8a70171bdd4e3d10a))
- Move Slice, ScalarStyle, StreamEncoding enums ([c2f20d7](https://github.com/dolysis/yary/commit/c2f20d7883ff649ee75276d80fcd5ad04b520ee5))
- Clippy lints ([8d20962](https://github.com/dolysis/yary/commit/8d20962640ad5774f300b0ec33f591e51e065887))
- Stub library documentation ([a8230f8](https://github.com/dolysis/yary/commit/a8230f86f26ff3a5e26284582a1f7367c85f93c0))
- Expose reader and event modules ([95eeec3](https://github.com/dolysis/yary/commit/95eeec30f53e34d4ac637b88854f63bdd03c08a3))
- Pin rust version to 1.53 ([f7d75b8](https://github.com/dolysis/yary/commit/f7d75b836fad3dc5f3db51e6c0348c162adde6ea))
- Derive Clone on more structs ([0343c29](https://github.com/dolysis/yary/commit/0343c29021543806c3d3ac85279a6cbf076d1d39))
- Fix visibility on Queue, Scanner, TokenEntry ([91545f4](https://github.com/dolysis/yary/commit/91545f4c7017dbd5c4495fe2bf51a6ea615723ee))
- Prune dead module reader ([da15105](https://github.com/dolysis/yary/commit/da15105e1d5d9daa84990a6663bcfcfebd180f41))
- Add atoi ([2768e5c](https://github.com/dolysis/yary/commit/2768e5c86c0c6b55061bc44ce4c77009a7e11059))
- Add scanner + token ([dee8a69](https://github.com/dolysis/yary/commit/dee8a69b719441865c4eb50dc373835403e67e26))

### Lib.Error

- Module doc ([d247b19](https://github.com/dolysis/yary/commit/d247b19068ecc9aa57968b751fc3eaf935cfd51e))
- Add mkError! macro ([c8496b0](https://github.com/dolysis/yary/commit/c8496b0216256a4e0382cf1cbcea3936547b201c))
- Add library error type for all public APIs ([ff1925f](https://github.com/dolysis/yary/commit/ff1925fdd0a04477f8ab8c43f2bb3afe2d5a3635))
- Use anyhow as an error stub ([b541afb](https://github.com/dolysis/yary/commit/b541afbf91d282cd178fa8425f622a9414ecb756))

### Lib.Event

- Fix doc link error ([6360463](https://github.com/dolysis/yary/commit/63604634351c6f1a4c8e8863b5301b2c2f9d55aa))
- Document all public items ([5c4d3c4](https://github.com/dolysis/yary/commit/5c4d3c4879484f3ddc99b76e1688f90a825c8790))
- Use array_iterator in tests macros ([fa84377](https://github.com/dolysis/yary/commit/fa84377ebba59ed99c1d04023363fa265f6f5e87))
- Hide internal types ([ab2c0fd](https://github.com/dolysis/yary/commit/ab2c0fd4387ed3e2f368bc058ed17c143b1b9d02))
- Use array_iterator() over ArrayIter::new ([51c7529](https://github.com/dolysis/yary/commit/51c7529f7519a4b923e74d5cc67f944831aca7ca))
- Use public Error type for returned errors ([4f402ee](https://github.com/dolysis/yary/commit/4f402ee38ecfb65083f21c482317303035e3c86c))
- Impl From local error type to public Error ([b5fa2ac](https://github.com/dolysis/yary/commit/b5fa2acc9e6d4f8e56324df5f066c1dc2809f5cb))
- Ensure private types are private ([62f9cf5](https://github.com/dolysis/yary/commit/62f9cf53e049a26ae0972ebf982d40d7be81828c))
- Add error conversion to ErrorKind ([2793e4c](https://github.com/dolysis/yary/commit/2793e4cc002e08833e30f9a69a6e971276c75ddc))
- Document ParseError variants ([ba41069](https://github.com/dolysis/yary/commit/ba41069dc6a070effab1fb0419037fc1866f183a))
- Expose public API for YAML event streams ([183dbc3](https://github.com/dolysis/yary/commit/183dbc3b1b2ce0a43c0951c24b7c0350731fd1c3))
- Add public Flags exposed to callers ([65f9908](https://github.com/dolysis/yary/commit/65f990872fec464106f5eab126fe30c18364a138))
- Use relative paths in test macros ([44759d4](https://github.com/dolysis/yary/commit/44759d458d0584245fe6c7ddd941f2752391a7a3))
- Module doc ([bdfafc0](https://github.com/dolysis/yary/commit/bdfafc057f1609bd3c6e1fdd97e2428e144e9573))
- Add module doc ([d2c25e2](https://github.com/dolysis/yary/commit/d2c25e2bd0d0c057a0798e03641ee325a1f834c3))
- Move Parser to lib/event/parser ([76824e9](https://github.com/dolysis/yary/commit/76824e9db78b6835383e1e5dd4004f0cc0a17afc))
- Add module documentation ([056b9e2](https://github.com/dolysis/yary/commit/056b9e27be3a4f7e7f5e7e43007dc47b5d729944))
- Add Parser tests ([fcfb870](https://github.com/dolysis/yary/commit/fcfb8705832d4a3d7e3d501ad9da427c8b11e143))
- Add tokens!, events!, event!, node!, scalar! ([2239a88](https://github.com/dolysis/yary/commit/2239a884fdab13c50f3652e2d228c48b86f8776c))
- Add handler for YAML nodes ([2024724](https://github.com/dolysis/yary/commit/2024724d04baa09d2a3f26b933012aa8b6628e0c))
- Add handlers for flow_sequence->mappings ([5dc521c](https://github.com/dolysis/yary/commit/5dc521c278843d4de7ab4271c57ef9a8a38b0a22))
- Add handlers for sequences/mappings ([34d893f](https://github.com/dolysis/yary/commit/34d893f4b347a80c834a9304c1c17fd2d9e5413e))
- Add handlers for YAML document state ([89c5b2d](https://github.com/dolysis/yary/commit/89c5b2df5e18fd2ac02b2ec7405f32c201e30c20))
- Add stream_start, stream_end, empty_scalar handlers ([d33c7da](https://github.com/dolysis/yary/commit/d33c7daf4e23a182632717171d1ebd2bb3c3f200))
- Add state handler skeletons ([36bf2fe](https://github.com/dolysis/yary/commit/36bf2fef52e0891c62bc8f7a62844dfae0849730))
- Add Parser, EventIter skeletons ([f086975](https://github.com/dolysis/yary/commit/f086975a636603e90d79dde09f5ecdb52269ddde))
- Add peek!, pop!, state!, consume!, initEvent! ([7634002](https://github.com/dolysis/yary/commit/763400291e1c27cfe62115147feceb31c41e901a))
- Add Event, EventData and child structures ([5ddbd93](https://github.com/dolysis/yary/commit/5ddbd93ce7f7a729fd07152d19d5314b41fcd229))
- Add module Error, Result typedef ([a21385e](https://github.com/dolysis/yary/commit/a21385e92c64bcdd42592280a819b7927394abb5))
- Add StateMachine, Flags ([0f6fb62](https://github.com/dolysis/yary/commit/0f6fb62cb71a1a650c6116b49d6686d0f9c4c58b))
- Add module stub ([19f294c](https://github.com/dolysis/yary/commit/19f294cb1ccdd9ab2a3dd4324db2d62547db2334))

### Lib.Queue

- Add Queue, a stable min binary heap ([7e567aa](https://github.com/dolysis/yary/commit/7e567aa8a91ccb5fd92f24c782112eb1585376da))

### Lib.Reader

- Fix doc link error ([10089e3](https://github.com/dolysis/yary/commit/10089e376be70a9cd64a95719fd3eba37300a4cd))
- Document module ([96d4531](https://github.com/dolysis/yary/commit/96d453105437ccdefbd0ec775b0b90ee6c40955e))
- Hide private types in Read methods ([742386b](https://github.com/dolysis/yary/commit/742386b177b5a19e2d4b7ca2dde92babc48c90bf))
- Add top level public API ([fe09d7a](https://github.com/dolysis/yary/commit/fe09d7aa277075876be9505abb3b8fee35c028eb))
- Impl From local error type to public Error ([4a48ac1](https://github.com/dolysis/yary/commit/4a48ac1735826ded5ef2a22339841136cb372039))
- Hide local error type ([46d088f](https://github.com/dolysis/yary/commit/46d088fa2dbbd3c1b107b8bd3af8a46c65e3274f))
- Add error conversion to ErrorKind ([519482f](https://github.com/dolysis/yary/commit/519482f79828d8d91e96f81457263560e47cde50))
- Fix visibility of public readers ([2e77556](https://github.com/dolysis/yary/commit/2e77556dd12ec263ef283b146cc58aeadf864f5b))
- Add test_reader! tests ([4911631](https://github.com/dolysis/yary/commit/49116317c1c2a9814679a58787bd7ad79ef79056))
- Add OwnedReader ([e1fe33e](https://github.com/dolysis/yary/commit/e1fe33e2024944d73cdc0662a192182ba143e053))
- Add test_reader! tests ([fb90078](https://github.com/dolysis/yary/commit/fb90078a3e8064d1d004952b2a83b3a745068a62))
- Add BorrowReader ([e49c473](https://github.com/dolysis/yary/commit/e49c47360456df2068290aa341922c3ed2e3fbd0))
- Add test_reader! macro ([8fa290d](https://github.com/dolysis/yary/commit/8fa290d374cd3fc4475bac95e5046e57eacf2e43))
- Add Reader, PeekReader structs ([43353da](https://github.com/dolysis/yary/commit/43353dae19d7a78807710c8251852b0f05404cc5))
- Add trait Read ([a08aab7](https://github.com/dolysis/yary/commit/a08aab7992e622d6351116053234f5f238029568))
- Comments ([832ef5e](https://github.com/dolysis/yary/commit/832ef5eb339adf61ab403ace5d83cea52ae1e540))
- Fix column @newline ([a09c983](https://github.com/dolysis/yary/commit/a09c983d56c68164fa4042e19e686b84a1a9ee4b))
- Fmt ([2e3562c](https://github.com/dolysis/yary/commit/2e3562c28e0a07d1fd8c7c234982da885a0c0002))
- Add unit tests ([95e1edd](https://github.com/dolysis/yary/commit/95e1edd1943771152948542eb8b1abc32df1a5fb))
- First go at a reader impl ([7c8b0cd](https://github.com/dolysis/yary/commit/7c8b0cdc43d131c004a781ecd4ff6b2956467ba9))

### Lib.Scanner

- Add From impl for ScanError to public Error ([76c5477](https://github.com/dolysis/yary/commit/76c547717d8b5f63d95cee666d8ae4aa661d4bf2))
- Make most types crate private ([04e0712](https://github.com/dolysis/yary/commit/04e07125e6722e90421e9ff23aef4430e9dda82d))
- Add error conversion to ErrorCode ([fba5f93](https://github.com/dolysis/yary/commit/fba5f934f578207ae12651a485f3dd319afca8f7))
- Fix visibility of modules ([a274144](https://github.com/dolysis/yary/commit/a2741447ac3bf000e9e36b5a1e8330ae60924cac))
- Add .marker() method ([a8a2aee](https://github.com/dolysis/yary/commit/a8a2aee615ecc227d5c39c34b4ea3afe5e3efa1d))
- Clippy lints from 1.56 ([bdbf510](https://github.com/dolysis/yary/commit/bdbf510a2461b899ec15924a69f6651a15d99a55))
- Add offset controls ([ce8b59b](https://github.com/dolysis/yary/commit/ce8b59b646795f0958c24ff9d98f6d8a0e4f85b6))
- Module documentation updates ([c977815](https://github.com/dolysis/yary/commit/c977815ddc8da0b480b7d947ad82d27589f8acbb)) <sup>scalar</sup>
- Fix subtle slice error in scan_plain_scalar_lazy ([9ed1bcc](https://github.com/dolysis/yary/commit/9ed1bcc00e860097f693a11c28e7c20ec2be2b80)) <sup>scalar</sup>
- Add unit test for escaped double quote ([1cdad01](https://github.com/dolysis/yary/commit/1cdad01126ba3048c093345ed29804f60e4783ee)) <sup>scalar</sup>
- Fixes to scan_flow_scalar_lazy's chomping ([0c38dda](https://github.com/dolysis/yary/commit/0c38dda908b2fed429b1b6b11230b15776ee28f9)) <sup>scalar</sup>
- Rename TEST_OPTS -> TEST_FLAGS ([0a4a793](https://github.com/dolysis/yary/commit/0a4a7930a5b6487e34b2564b8d54e78ed47bf39e)) <sup>scalar</sup>
- Update ScanIter to use TEST_FLAGS always ([9c75400](https://github.com/dolysis/yary/commit/9c75400697618e59700753470cd41bfb447101a2))
- Add test_flags and const TEST_FLAGS ([f8dc375](https://github.com/dolysis/yary/commit/f8dc375d1446b60cbf63f78cb7dfdd3622010a03))
- Add Indent.as_usize ([522d38b](https://github.com/dolysis/yary/commit/522d38b66582891c5cabf7353228eb079e85cb47))
- Refactor shared functions/consts ([4c63c0c](https://github.com/dolysis/yary/commit/4c63c0c047a5fc9d412694d73d7f5709fb37ee6b)) <sup>scalar</sup>
- Fix tests ([bfdb0d7](https://github.com/dolysis/yary/commit/bfdb0d78a7414448dcd882ae11a48d5f093b174d)) <sup>scalar</sup>
- Add ScalarB variant to MaybeToken for block scalars ([64adcb1](https://github.com/dolysis/yary/commit/64adcb10b7d23ae7fea57b9a59ce5c4c5b010d57))
- Add scan_block_scalar_lazy, return MaybeToken ([3414434](https://github.com/dolysis/yary/commit/34144344fce58cab8aefff3400fb04c143b0f469)) <sup>scalar</sup>
- Fix tests ([fc669a6](https://github.com/dolysis/yary/commit/fc669a6e92b04dea19451139225af55ac28c0765)) <sup>scalar</sup>
- Add ScalarP variant to MaybeToken for plain scalars ([3e6e04c](https://github.com/dolysis/yary/commit/3e6e04c3b287a3f0bf5e353eee3a44d203ebc47a))
- Add scan_plain_scalar_lazy, return MaybeToken ([5786cc1](https://github.com/dolysis/yary/commit/5786cc159f3149a66444060c4a11603643398d0d)) <sup>scalar</sup>
- Fix tests ([0f7d65e](https://github.com/dolysis/yary/commit/0f7d65ee7dd52a4b114967b202d93a4dfca0970e)) <sup>scalar</sup>
- Add scan_flow_scalar_lazy, return a MaybeToken ([0268ccf](https://github.com/dolysis/yary/commit/0268ccf46321aec359e49c8bbe0e1835bc74929c)) <sup>scalar</sup>
- Add MaybeToken wrapper to allow for deferred tokens ([6880c67](https://github.com/dolysis/yary/commit/6880c673dd48b2e873e74a4869b3da3b070260ce))
- Normalize scan_flow_scalar's return value ([6259a31](https://github.com/dolysis/yary/commit/6259a31c66a297a5df4de752fd243b982549d098)) <sup>scalar</sup>
- Add feature gated test harness for tokens! ([68d3de8](https://github.com/dolysis/yary/commit/68d3de83423080a7aa0c01196a47f256549abf03))
- Place state mutation after any O_EXTENDABLE events ([550bff9](https://github.com/dolysis/yary/commit/550bff999be32313a25d57db8a7e60bbfca5304e))
- Save any changes that may occur after a ScanError::Extend ([4b79e98](https://github.com/dolysis/yary/commit/4b79e98048b510582cbdd1768d7e30f6eddf437b))
- Clippy ([3856f69](https://github.com/dolysis/yary/commit/3856f69a84cceffcf2d1d514fe621a6bea90c998))
- Cache! before fetch ([5e7f349](https://github.com/dolysis/yary/commit/5e7f3498036566cedf5667783901323aa83abfd1)) <sup>scalar</sup>
- Cache! before fetch ([836716f](https://github.com/dolysis/yary/commit/836716f5a3851b9a9839f081c775793b4448b058)) <sup>scalar</sup>
- Cache! before fetch ([1f22f9d](https://github.com/dolysis/yary/commit/1f22f9d609074f4c2f073e1380bd197d1c88d7df))
- Fix tests ([564ee14](https://github.com/dolysis/yary/commit/564ee1476ec6c7bbfa2c639e71f4284e0b0d0797)) <sup>scalar</sup>
- Cache! before fetch ([f1fa8a6](https://github.com/dolysis/yary/commit/f1fa8a6620176de6cffe10280d024e3e77fa5029)) <sup>scalar</sup>
- Cache! before fetch ([13ff795](https://github.com/dolysis/yary/commit/13ff795bf36b9e1312dfd698215648707e331828))
- Cache! before fetch ([6d82e8b](https://github.com/dolysis/yary/commit/6d82e8b045786e1f4cf59ca6241f68fd463b4825))
- Cache! before fetch in scan_next_token ([86cc5e7](https://github.com/dolysis/yary/commit/86cc5e72d9edf49ade0050b9df3f56bf77023898))
- Add opts to scan_tokens, eat_whitespace cache! ([69e202a](https://github.com/dolysis/yary/commit/69e202a4cbd06938f3a2f2292fd85eb1c486ae99))
- Add cache! ([9b13d54](https://github.com/dolysis/yary/commit/9b13d54e44a24ddea71823207747c6aeead765f3))
- Add variant Extend ([0663beb](https://github.com/dolysis/yary/commit/0663bebd0c93f0e49757f14ab5e7cd2d4fae4dd5))
- Add Flags for Scanner control ([0b023bd](https://github.com/dolysis/yary/commit/0b023bd062fe958fdc93477dd5f832ef0c3bb47c))
- Prune dead documentation ([82a6e70](https://github.com/dolysis/yary/commit/82a6e70d8be4c992ce9968e365719692077e6262))
- Move Scanner.eat_whitespace out of fetch_* methods ([c24f9ef](https://github.com/dolysis/yary/commit/c24f9ef286df4a1c8d454b38b0fe079cb0860d1c))
- Rename Scanner token retrieval methods to fetch_* ([2afc2b2](https://github.com/dolysis/yary/commit/2afc2b26067193fae062a2d9cd7efa250c5c8809))
- Move test code into scanner/tests ([0f76f9b](https://github.com/dolysis/yary/commit/0f76f9bb081ca276606d92d690768b4c6965a246))
- Refactor anchor scanning into its own module ([633e461](https://github.com/dolysis/yary/commit/633e461f4a37eb97f1460c2585145cca7764de68))
- Merge crate:: and self:: use statements ([829f5c0](https://github.com/dolysis/yary/commit/829f5c0e8104d3f9d8f9a7969493ad2ff89191a5))
- Move directive scanning to a separate module ([4bc2eb5](https://github.com/dolysis/yary/commit/4bc2eb5c9f8be9587b51ca7a7eda818300c7eaf9))
- Move MStats into its own module ([842ed7c](https://github.com/dolysis/yary/commit/842ed7cacb49cdfb756e34f762792ada563f68ec))
- Use const indicators over byte literals ([1ed9d45](https://github.com/dolysis/yary/commit/1ed9d453449f5d96173ad0736b00d04b49b3a34c))
- Refactor tests ([0b343ff](https://github.com/dolysis/yary/commit/0b343ffa720b02d93ad6ef58bc6cafee304fc028))
- Unit tests for block scalar token streams ([369f49a](https://github.com/dolysis/yary/commit/369f49a248d7d334ea92033b3277598b986f0f24))
- Add catch all error, documentation ([1f144ca](https://github.com/dolysis/yary/commit/1f144caf76966fef230fb83dbaf64123bdc87e4c))
- Add support for block scalars ([b26301c](https://github.com/dolysis/yary/commit/b26301cbd04d9358d1d8c91cee68f94fbc0433f3))
- Add variant UnknownDelimiter ([0f42818](https://github.com/dolysis/yary/commit/0f42818f4b0c063701eaca1d2d77acc632513a62))
- Add unit test for header comments ([8f4cab4](https://github.com/dolysis/yary/commit/8f4cab4f5bfe4f835f43a8cd5ca0ae1598e6a76b)) <sup>scalar</sup>
- Fix skip_blanks comment handling ([87af3bc](https://github.com/dolysis/yary/commit/87af3bc96b91c75410c1a9ab4ff76b836f18e541)) <sup>scalar</sup>
- Clippy lints ([8505569](https://github.com/dolysis/yary/commit/8505569d3e31ffcbbe0f2b7157ea62b5f29a8785)) <sup>scalar</sup>
- Code reorganization ([6450079](https://github.com/dolysis/yary/commit/645007938ff054699d7959e4643c9ba4a79611f7)) <sup>scalar</sup>
- Documentation ([4ec311d](https://github.com/dolysis/yary/commit/4ec311d68430228de2cf1408577cf6594627e45c)) <sup>scalar</sup>
- Add unit tests for scan_block_scalar ([0ffccab](https://github.com/dolysis/yary/commit/0ffccab6c313cb719d3279e845cee697c9b276a1)) <sup>scalar</sup>
- Add scan_block_scalar ([bcf1e40](https://github.com/dolysis/yary/commit/bcf1e405ecdbaf7bd23ee12c5cd0c2df9ddbc6cf)) <sup>scalar</sup>
- Add widthOf! ([dba9212](https://github.com/dolysis/yary/commit/dba9212224d04b54a419b564e305007fb02522c1))
- Add isBreakZ! ([3318f87](https://github.com/dolysis/yary/commit/3318f8762a8cd1c6cf525a7e16916efbcdd5d8df))
- Add InvalidBlockScalar, InvalidTab variants ([1b517b5](https://github.com/dolysis/yary/commit/1b517b518faf932fd1660b6bf5e24ecd4ba16f2c))
- Add complex test for plain scalars ([4eced7d](https://github.com/dolysis/yary/commit/4eced7d9f4bb8f4f2feda139a034f1a4c41e9142))
- Add test for YAML indicators in plain scalar ([4189d3d](https://github.com/dolysis/yary/commit/4189d3db96060faf14345616d0028c0a1eb2073d))
- Fix indentation level to account for the 0'th level ([76d9ec5](https://github.com/dolysis/yary/commit/76d9ec5561618fd12e2fe9ba442ad0c47efb4f48)) <sup>scalar</sup>
- Fix is_plain_scalar to block unsafe plain chars ([338db9c](https://github.com/dolysis/yary/commit/338db9ce420e00da8a13c36e9ddd75f0de085254))
- Clippy lints ([28a7ce9](https://github.com/dolysis/yary/commit/28a7ce919109b99cda3b62000e5aacb37dd51c07))
- Add unit tests for plain scalar token sequences ([048550a](https://github.com/dolysis/yary/commit/048550a7b1520471d8257bd20db897ce4e32148e))
- Add support for plain scalars ([5d8f78b](https://github.com/dolysis/yary/commit/5d8f78be253711a268c116aeaa47de62ef84f94e))
- Fix handling of non EOF trailing whitespace ([88ac017](https://github.com/dolysis/yary/commit/88ac017647729ce928debee642766d4723d80d5a)) <sup>scalar</sup>
- Add unit tests for scan_plain_scalar ([b3e86de](https://github.com/dolysis/yary/commit/b3e86dea9c35775db64c37b1a749d0b91715e4ec)) <sup>scalar</sup>
- Add scan_plain_scalar ([cddc1da](https://github.com/dolysis/yary/commit/cddc1dae09bba13c08311418241c2dfd3cb49b97)) <sup>scalar</sup>
- Use isDocumentIndicator! over longhand ([2aae676](https://github.com/dolysis/yary/commit/2aae6760f55027b5879d205ebe786cb64fbec6fe)) <sup>scalar</sup>
- Add isDocumentIndicator! ([0fcc614](https://github.com/dolysis/yary/commit/0fcc61477137caa9eb036da6086a292d67e30c97))
- Add variant InvalidPlainScalar ([7d600cd](https://github.com/dolysis/yary/commit/7d600cd29e54225a0454c662939b21dc4481f962))
- Clippy lints ([ce7acbb](https://github.com/dolysis/yary/commit/ce7acbb7545fb195ce0ab0e8f9df64b576acbc05))
- Add tests for explicit key cases ([71266f1](https://github.com/dolysis/yary/commit/71266f1530996a7ea20ccc8f32a95ed5b481c6ff))
- Add explicit key support to Scanner ([8558dad](https://github.com/dolysis/yary/commit/8558dada841a4f2ded69ea99c38a263c985c3218))
- Add variant InvalidKey ([4c61af7](https://github.com/dolysis/yary/commit/4c61af7eb955c7045896db141efd516ac3b7b286))
- Add test for zero indented sequence decrement ([76be700](https://github.com/dolysis/yary/commit/76be7001bb2aacba28b1ccfedab79aeb7dde761a))
- Further fixes to zero indented sequence handling ([5d0572d](https://github.com/dolysis/yary/commit/5d0572d02d343e6fb821fc4110a774d4f32a2d83))
- Produce token for zero indentation block sequence ([18d6430](https://github.com/dolysis/yary/commit/18d6430cc228099376d8b9618dbad18a4e250823))
- Clippy lints ([76acbeb](https://github.com/dolysis/yary/commit/76acbeba93dcf1c0c63d12c3c4dad6b9096b12d3))
- Add tests for catching expected errors in stale keys ([29afcda](https://github.com/dolysis/yary/commit/29afcda3eec136fdbe8ef20fe2e87ebaaf291da2))
- Fix Scanner.value ignoring key state ([cbde7cc](https://github.com/dolysis/yary/commit/cbde7ccb91bcf2d3bcda0c3a55116a56a31fcfad))
- Add tests for block collections ([72e38d2](https://github.com/dolysis/yary/commit/72e38d21009e2fc7208b08cc4a5801d8da172ca2))
- Add support for BlockEntry tokens to the Scanner ([5d6a023](https://github.com/dolysis/yary/commit/5d6a0230778d73441ef2ed617aa6fa62ca58f645))
- Update tests to expect BlockEnd tokens ([5ba2161](https://github.com/dolysis/yary/commit/5ba216147d6240bd3da33b6bc16d7b8a03232b1e))
- Update scanner code to decrement indent ([8773625](https://github.com/dolysis/yary/commit/8773625259b04e3cc1fbe5aa40d7d9ac9dacd72a))
- Allow passing indents directly to indent_decrement ([e5b1bb0](https://github.com/dolysis/yary/commit/e5b1bb0dac3bc079d93127c6a391469286666366))
- Document Scanner.value, allow bare ':' in flow context ([90d2ac8](https://github.com/dolysis/yary/commit/90d2ac8a060340e2d16aaa38bec98edc07b26d3c))
- Move saved key check into value function ([349e62b](https://github.com/dolysis/yary/commit/349e62be0af21b8dc1b8af1b7286c9cf74077003))
- Add expire_stale_saved_key check ([458f806](https://github.com/dolysis/yary/commit/458f806055f8df3754fc9139911a68285a62b3b7))
- Add InvalidValue ([46b616e](https://github.com/dolysis/yary/commit/46b616ea6f0136dfbc14d4b81edcde9f9a0f69fa))
- Remove duplicate check from Scanner.value ([ce4deb7](https://github.com/dolysis/yary/commit/ce4deb76fa5fa168a8c0bef557823d959ee40c92))
- Fix tests ([b5fa29c](https://github.com/dolysis/yary/commit/b5fa29ca099bec10b33e321d3d1df8409e0dfa86))
- Ensure Scanner does not exit before Key resolution ([6825e3e](https://github.com/dolysis/yary/commit/6825e3ebd4addb910ace5c1bbd00f95d106bd762))
- Remove dead code ([ebda074](https://github.com/dolysis/yary/commit/ebda074d667d05b45a835e4c913d1543ceb6c9af))
- Update block_collection_entry roll_indent call ([0a36fc9](https://github.com/dolysis/yary/commit/0a36fc9e6ecf152436d0badb76aa6314fb18a0f0))
- Update ScanIter for Queue based token stream ([f8f8536](https://github.com/dolysis/yary/commit/f8f8536631de9584c78e62ea9586cf18743d2bad))
- Fix flow_scalar Key check ([5935ba0](https://github.com/dolysis/yary/commit/5935ba0056b179f61e1b51523c5c08cf69b78fd2))
- Fix un/roll_indent function defs ([1ac7eb5](https://github.com/dolysis/yary/commit/1ac7eb556b786a234457c008c76980ef72a2d581))
- Save keys across Scanner ([0ee871d](https://github.com/dolysis/yary/commit/0ee871dad59c532e07b59553af47e7ff0ac6dff8))
- Add save_key, remove_saved_key ([01039b6](https://github.com/dolysis/yary/commit/01039b62e2dbeb894f6a831eb82ecfdc32d4a511))
- Enqueue! tokens ([aa7214e](https://github.com/dolysis/yary/commit/aa7214ee3571b680b02950d91864d297bd2a7d2b))
- Use simple_key_allowed over self.key_* ([971e2c7](https://github.com/dolysis/yary/commit/971e2c76d4b5e5edda6f4e5c9a4771a6d038761c))
- Switch Tokens->Queue<TokenEntry>, add Scanner.simple_key_allowed ([cda72c5](https://github.com/dolysis/yary/commit/cda72c58fa244338d61c54a22ac18f017000f591))
- Refactor ([87640bd](https://github.com/dolysis/yary/commit/87640bd1f462918ed6ffb6a72806b92afc8729c6))
- Add enqueue! ([a82aa3c](https://github.com/dolysis/yary/commit/a82aa3c35d8b9e01433fdbb5b1ff83e6e87cb2a8))
- A custom Ord Token wrapper ([840868f](https://github.com/dolysis/yary/commit/840868f82ef8f686495d4d4c06acc90542776403))
- Add tests for flow contexts ([5212077](https://github.com/dolysis/yary/commit/5212077ae8c6acd7ea5ab0cb8817b36985e4288b))
- Add test for simple flow sequence ([24a8f2b](https://github.com/dolysis/yary/commit/24a8f2b2113fb852d66dd2dced21ec52bf68d8d0))
- Fix flow de/increment ([9e295b8](https://github.com/dolysis/yary/commit/9e295b8f72339a2991ed9e9eec72c3a195dff39f))
- Add flow/block entry scan functions ([37221ad](https://github.com/dolysis/yary/commit/37221ad0201ae89d3a685946cf8f5bc9c111a8cf))
- Add un/roll_indent functions ([6b89652](https://github.com/dolysis/yary/commit/6b8965268c48e232de2b39647ef207ef2a71183b))
- Add InvalidBlockEntry ([5ec3d0a](https://github.com/dolysis/yary/commit/5ec3d0ae2b463abf47f78b8f19308d69228807e5))
- Add unit tests for flow_collection_* methods ([e24fe38](https://github.com/dolysis/yary/commit/e24fe38a7ef56371f7abe2cc7f579d3a96686260))
- Track YAML context, add flow_collection_* methods ([a84f64e](https://github.com/dolysis/yary/commit/a84f64e2b728e720f944d5755362756f0af4226c))
- Add Context ([c2734e3](https://github.com/dolysis/yary/commit/c2734e33e1c3042f0c79a9e2b9f85c9e9cd975eb))
- Add IntOverflow variant ([9347073](https://github.com/dolysis/yary/commit/93470734ba4f7963d4f7221c8d6e5c081e470bbe))
- Rename key.impossible -> key.forbidden ([d32fc77](https://github.com/dolysis/yary/commit/d32fc77b175c8ac3fc9c262e93bf44a2dbd0d9ce))
- Fix check_is_key, correct test ([e35a1e8](https://github.com/dolysis/yary/commit/e35a1e87dbf26ca018b351ad04e16e5a938f9ead))
- Clippy, fmt ([dd04944](https://github.com/dolysis/yary/commit/dd04944fe9888b19fa00447eeba8da7d0f1a9d96))
- Remove old code ([eee164c](https://github.com/dolysis/yary/commit/eee164caa1174e02131d7942c5b6d8e9fc71d627))
- Allow multi token calls ([b1713c7](https://github.com/dolysis/yary/commit/b1713c73f6af2dee48dd82cdff4daa4dd7e8ba80))
- Adjustments for the API changes ([53a8c8e](https://github.com/dolysis/yary/commit/53a8c8eccb0f1cf7c026fdec202d3b4203a349f7))
- Remove ref, return owned Slice variants ([8986f36](https://github.com/dolysis/yary/commit/8986f36f003480cdd43de2e4e18b2617dc5a0ea0))
- Tidy syntax / includes ([8f84972](https://github.com/dolysis/yary/commit/8f84972cd57c21d02541592feeea6875d6123491)) <sup>scalar</sup>
- Save the scanned scalar's stats ([cd7859f](https://github.com/dolysis/yary/commit/cd7859f3d498e8a24a19ba464d74cd49ca828a5f))
- Remove reset_stale_keys, dbgs ([696b71b](https://github.com/dolysis/yary/commit/696b71b083ec249279f44f7bc322c2ff9e48c1c4))
- Add tests to catch trailing ws bugs ([7857839](https://github.com/dolysis/yary/commit/7857839d6c675a512918ffdcab2748faef4a2877))
- Bugfix always count whitespace ([e416e3d](https://github.com/dolysis/yary/commit/e416e3d0abf754c70ff009e266c98f32109ca75b))
- Add value token scanner, track keys ([a595279](https://github.com/dolysis/yary/commit/a59527944ba4cda4417a9d863a75654840db100d))
- Add, structs for managing key tokens ([a0e1844](https://github.com/dolysis/yary/commit/a0e184431f4b03fe5458c67f200a0e1de1c3bb09))
- Return ScalarRange over Ref ([81975e1](https://github.com/dolysis/yary/commit/81975e197f9464ce77627720a4d6120e7d08aa97))
- Add test for implicit key ([cebd1d6](https://github.com/dolysis/yary/commit/cebd1d6e7dc76610b57640b2991c6a7e640bd4f2))
- Fix primary branch in scan_node_tag ([17f09e4](https://github.com/dolysis/yary/commit/17f09e4d3022d26f7a22d75a5117001a93689ff3))
- Document MStats ([298b15c](https://github.com/dolysis/yary/commit/298b15cad7b5e8ee6f969116617254b2d5bf05c3))
- Add stats test to unit tests ([6490de8](https://github.com/dolysis/yary/commit/6490de8974d63eb061909e863445f2cee0626b27))
- Fix unit tests ([393ce63](https://github.com/dolysis/yary/commit/393ce6372ba4725bd651662f06607be6e31ada85)) <sup>scalar</sup>
- Track stats in anchor ([32ada94](https://github.com/dolysis/yary/commit/32ada9485072b4259a5a49f64b19537d88679581))
- Track stats in version directive ([46fcd61](https://github.com/dolysis/yary/commit/46fcd61aec3e6a808b94d2e4d72f11b4f5967ca5))
- Track stats in document_marker ([911f861](https://github.com/dolysis/yary/commit/911f86132086ce8b31a94a95bd6767fa04d53327))
- Track stats in eat_whitespace ([f2399e0](https://github.com/dolysis/yary/commit/f2399e0eb413c71d633d46902b970727eadbd345))
- Allow advance! to optionally update :stats ([69574b3](https://github.com/dolysis/yary/commit/69574b3628fdee6d321f5fa54c01e6ab893e17a3))
- Add MStats ([cb6d64d](https://github.com/dolysis/yary/commit/cb6d64dfc71ba5296f63a65cf02e4f5825dc5168))
- Fix tokens! ScanIter lifetimes ([8be1ca8](https://github.com/dolysis/yary/commit/8be1ca8329ebbda1f3df1815113debdfe9887c9a))
- Clippy lints ([af8b18c](https://github.com/dolysis/yary/commit/af8b18c781cca771c3eedb82428e35900cf7d0e5))
- Add unit tests for tag + flow scalar scanning ([5eb1e73](https://github.com/dolysis/yary/commit/5eb1e739c0762cd5cd29c9e983e1bebf0d7e7226))
- Add flow scalar to next_token ([13d7091](https://github.com/dolysis/yary/commit/13d7091939964415c5d5ffffe7b1f50eaff54515))
- Add tag scan to next_token ([26cd164](https://github.com/dolysis/yary/commit/26cd16403b169a243a0bc629da838f4a21617d92))
- Make scan_flow_scalar public in lib/scanner ([e5dda14](https://github.com/dolysis/yary/commit/e5dda1467da1e02507a97fefad5d745a3c71f902)) <sup>scalar</sup>
- Refactor tag directive scan to use scan_tag_directive ([2654799](https://github.com/dolysis/yary/commit/2654799739a202bf9107e03941bd1424f2dd1e1c))
- Add scan_tag_directive, scan_node_tag ([09fc128](https://github.com/dolysis/yary/commit/09fc1285454aabc1d8575af7dcd5b224f59be65d))
- Add InvalidTagSuffix variant ([1f2f9b5](https://github.com/dolysis/yary/commit/1f2f9b507e39a95d551b77983aaa628325dbcd2f))
- Refactor eat_whitespace into a free function ([07fda2c](https://github.com/dolysis/yary/commit/07fda2c8a20e64cf212437e9932a95e838076733))
- Add advance! @line variant ([9db21c0](https://github.com/dolysis/yary/commit/9db21c0e232061c5c6dd312b5f1f65283492866f))
- Refactor tag directive scanner to use scanner/tag functions ([157040b](https://github.com/dolysis/yary/commit/157040bff04cd486ace803322dba94f7690ca3d2))
- Add scan_tag_uri, scan_tag_handle ([f14a843](https://github.com/dolysis/yary/commit/f14a84354919493c3b43091e52f080982a755646))
- Update isBlankZ! -> isWhiteSpaceZ! ([206ef90](https://github.com/dolysis/yary/commit/206ef90575f0372b75add3d3151e20d821f82751)) <sup>scalar</sup>
- Rename isBlankZ! -> isWhiteSpaceZ!, add isWhiteSpace! ([6a4649c](https://github.com/dolysis/yary/commit/6a4649c10f482f451860462d43f4b8df6ba9982e))
- Clippy fixes ([95af7eb](https://github.com/dolysis/yary/commit/95af7eb5b00948c9eaba5b6dd37036dd40a9abae))
- Add unit test for tag directive escapes ([89b7480](https://github.com/dolysis/yary/commit/89b7480cde6cb58c685f8c9252156fa0b83c1dff))
- Update tokens! to use ScanIter ([9a29c29](https://github.com/dolysis/yary/commit/9a29c29f5932a650fb41fc8957a6dbc6fd547632))
- Rewrite tag directive scan, return Ref over Token ([2dd5042](https://github.com/dolysis/yary/commit/2dd5042fd6f47153b9f1fb0413a3c071b6bb8cf8))
- Make submodules public (to scanner) ([d6f0c71](https://github.com/dolysis/yary/commit/d6f0c71e71fc5b33d3ec876c49be2bbbeea6ff67))
- Clippy lints ([da9e4f1](https://github.com/dolysis/yary/commit/da9e4f14e8903507a0983a2036aa47f07787325f)) <sup>scalar</sup>
- Add more unit tests for tag_uri_unescape ([e1d79d4](https://github.com/dolysis/yary/commit/e1d79d4851d98cc43bacd354a96b5b202c4b6ba4)) <sup>scalar</sup>
- Fix flow_unescape documentation ([5be9c9f](https://github.com/dolysis/yary/commit/5be9c9fb1d8db1c7b0ada4429d4c591198d18d14)) <sup>scalar</sup>
- Move exported fns to top, document tag_uri_unescape ([54614aa](https://github.com/dolysis/yary/commit/54614aafd050752f41ef6858821c98c62b1fba15)) <sup>scalar</sup>
- Add tag_uri_unescape ([cb7d622](https://github.com/dolysis/yary/commit/cb7d622b980eadbe1690a419e2ccf131367f0790)) <sup>scalar</sup>
- As_hex returns u8 ([fb5619c](https://github.com/dolysis/yary/commit/fb5619c6382c35a0ffc56aadd01b179d8de1c338)) <sup>scalar</sup>
- Add isHex! ([b00970e](https://github.com/dolysis/yary/commit/b00970e2f4a1cd428a5cd2bc180f070f50b9f808))
- Fix error path syntax ([56ea9f6](https://github.com/dolysis/yary/commit/56ea9f6921f523948746e70e2253c80b527ca51f))
- Implement double quote handling ([b4245b0](https://github.com/dolysis/yary/commit/b4245b0936df9b358c28809cb83c8d7ed351f034))
- Implement line break handling/joining ([fa89882](https://github.com/dolysis/yary/commit/fa8988213e80a6120cd58cd961c7b976047b366f))
- Check! handle EOF checks gracefully ([fd409a8](https://github.com/dolysis/yary/commit/fd409a8f20a6da1f5fbac297a14f1e6fd9f4f2d0))
- Standardize documentation ([ab668c7](https://github.com/dolysis/yary/commit/ab668c7e3e113a9999c6468f715e1fe1b378c094))
- Further improvements ([3b012f4](https://github.com/dolysis/yary/commit/3b012f461dbde12cd3d9d3851affce3631e10ce9))
- Various touchups ([4dc5efa](https://github.com/dolysis/yary/commit/4dc5efae33ded5528f976e88c06f0c149ae75458)) <sup>scalar</sup>
- Add flow_double_unescape, unit tests ([a750275](https://github.com/dolysis/yary/commit/a750275b47c8fa18a79ed951487c2098634e64b4)) <sup>scalar</sup>
- Implement skeleton for flow scalar scanning ([5117961](https://github.com/dolysis/yary/commit/5117961b23d0316dabde4cc397e3816de91daf74))
- Add unit tests for isBlank!, isBreak!, isBlankZ! ([9cf1600](https://github.com/dolysis/yary/commit/9cf16000884b12884357c81b0fb0894fc8349254))
- IsLineBreak! -> isBreak, add isBlank!, isBlankZ! ([c478e26](https://github.com/dolysis/yary/commit/c478e26281ffa9bae28a4c84a95bad1a75787246))
- Rewrite check! to be simpler to use, add isLineBreak! ([ba10beb](https://github.com/dolysis/yary/commit/ba10beb0c6228d87dbca0dd4d58a8555c82247aa))
- Allow advance! to update a var with $amount consumed ([3526cd2](https://github.com/dolysis/yary/commit/3526cd29c4c3686eef47665d83c3707b2e1cc620))
- Add InvalidFlowScalar, UnknownEscape variants ([aa2dec0](https://github.com/dolysis/yary/commit/aa2dec094e7b0cfab1dc4f01b2dd0dc21d0493bc))
- Fix macro propagation to submodules ([4bf4b50](https://github.com/dolysis/yary/commit/4bf4b50e3d678362dd1f04b94bfa0e28d51e0735))
- Allow YAML anchors (*ref, &ref) to be tokenized ([93e3ff8](https://github.com/dolysis/yary/commit/93e3ff8f769746423f044a0f3ce7d0aeef6cd065))
- Add InvalidAnchorName error variant ([f6f7102](https://github.com/dolysis/yary/commit/f6f7102a16509776a25af096c2ac1ed54407e6a0))
- Clippy lints ([8ce016f](https://github.com/dolysis/yary/commit/8ce016f3200403b7badde6b7a3213e3c0db53ac1))
- Scan tag directives ([5240cb7](https://github.com/dolysis/yary/commit/5240cb73306c78c5c30492c7b4f9c280fb865374))
- Mv cow! to normal macros, add check! ([a66664d](https://github.com/dolysis/yary/commit/a66664dd27758e032bc93c950b49766a65365fce))
- Add InvalidTagHandle, InvalidTagPrefix variants ([e852509](https://github.com/dolysis/yary/commit/e852509921b322bc4f3209b173294de3f1bf11e1))
- Improve version directive parsing ([2596a79](https://github.com/dolysis/yary/commit/2596a79e3fbdceb59ce86e8ef8131e714320ca9f))
- Add UnexpectedEOF error variant ([8c453af](https://github.com/dolysis/yary/commit/8c453afcb1c44f01d01126b5a22516d86f91fd99))
- Add tokens! > variant for matching Results ([3a93cb0](https://github.com/dolysis/yary/commit/3a93cb006a8ddfc41063b97a512f0a278bc6188c))
- Clippy lints ([3348cd8](https://github.com/dolysis/yary/commit/3348cd8fedca397d40a1617e4c809437fae76721))
- Add support for version directive ([ea21c08](https://github.com/dolysis/yary/commit/ea21c0824f76ef5922a53eb4d41d6bf4f4137dd1))
- Add error::ScanError ([7beda49](https://github.com/dolysis/yary/commit/7beda497d0ea53b827161735ed8c65d148b0934f))
- Improve eat_whitespace to chomp comments ([85c9a9f](https://github.com/dolysis/yary/commit/85c9a9f0c5c8af15e4b4fb8904626ed9ec5731ff))
- Improve tokens! macro error reporting ([68cc3f7](https://github.com/dolysis/yary/commit/68cc3f7342ce1f4223d4d6ecf43feaf34adc82cd))
- Add fn for chomping whitespace between tokens ([9ecea20](https://github.com/dolysis/yary/commit/9ecea201c86c857b3d712046c0a0d808af376e55))
- Add Scanner struct + tokens! macro ([bdc7031](https://github.com/dolysis/yary/commit/bdc70312f1d25912e1aa2da6809872565a1401df))

### Lib.Token

- Cmp with marker by ref (clippy) ([0fe2e99](https://github.com/dolysis/yary/commit/0fe2e99426ef13da81e1155c665983488821ef83))
- Add marker ([ea64559](https://github.com/dolysis/yary/commit/ea64559444b742562ef3f8a938011bf45d3f0ed0))
- Derive clone on style ([ffcdce5](https://github.com/dolysis/yary/commit/ffcdce59616767e1b08da6e9484413a5b1e13f8d))
- Add helper methods to Token + Ref ([cd3e7be](https://github.com/dolysis/yary/commit/cd3e7beb1ec1628887d799587fbc70af9fd070fd))
- Move Ref to token ([c4e514f](https://github.com/dolysis/yary/commit/c4e514f6c2b83cbc96cd8f5c3555d6d96f1d8daf))
- Fix mixup in directive variants ([be4c6ad](https://github.com/dolysis/yary/commit/be4c6ad52c88ef4d99814a2cb49fd199c61f83e0))
- Add Token enum of possible tokens emitted ([f4608a8](https://github.com/dolysis/yary/commit/f4608a858863c6fcb66807a00ab33903aad433dc))

### Style

- Apply rustfmt rules ([2146f55](https://github.com/dolysis/yary/commit/2146f55f2806b448ca5586160ccdc678e571d222))
- Add .rustfmt to root ([8dc9d10](https://github.com/dolysis/yary/commit/8dc9d10dfefd22f726f5333311a226c7a6b81266))

### Git

- Ignore .vim/ ([70d2f3a](https://github.com/dolysis/yary/commit/70d2f3a2e5a98a85c35bf3ef2f4ecaaa1f86bbe5))

## [v0.0.0 - 2021-06-01](https://github.com/dolysis/yary/releases/tag/v0.0.0)

<!-- generated by git-cliff -->
