/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract build', () => {
	it('should build contract templates', () => {
		const contracts = Contract.build({
			name: 'Debian {{this.data.codename}}',
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			data: {
				codename: 'Wheezy',
				url: 'https://contracts.org/downloads/{{this.type}}/{{this.slug}}/{{this.version}}.tar.gz',
			},
		});

		expect(contracts).to.deep.equal([
			new Contract({
				name: 'Debian Wheezy',
				slug: 'debian',
				version: 'wheezy',
				type: 'sw.os',
				data: {
					codename: 'Wheezy',
					url: 'https://contracts.org/downloads/sw.os/debian/wheezy.tar.gz',
				},
			}),
		]);
	});

	it('should support slug and type templates', () => {
		const contracts = Contract.build({
			name: 'Debian Wheezy',
			slug: '{{this.data.slug}}',
			version: 'wheezy',
			type: '{{this.data.type}}',
			data: {
				slug: 'debian',
				type: 'sw.os',
			},
		});

		expect(contracts).to.deep.equal([
			new Contract({
				name: 'Debian Wheezy',
				slug: 'debian',
				version: 'wheezy',
				type: 'sw.os',
				data: {
					slug: 'debian',
					type: 'sw.os',
				},
			}),
		]);
	});

	it('should expand contract variants', () => {
		const contracts = Contract.build({
			slug: 'debian',
			type: 'sw.os',
			variants: [
				{
					version: 'wheezy',
				},
				{
					version: 'jessie',
				},
				{
					version: 'sid',
				},
			],
		});

		expect(contracts).to.deep.equal([
			new Contract({
				slug: 'debian',
				version: 'wheezy',
				type: 'sw.os',
			}),
			new Contract({
				slug: 'debian',
				version: 'jessie',
				type: 'sw.os',
			}),
			new Contract({
				slug: 'debian',
				version: 'sid',
				type: 'sw.os',
			}),
		]);
	});

	it('should build contracts with variants and templates', () => {
		const contracts = Contract.build({
			name: 'debian {{this.version}}',
			slug: 'debian',
			type: 'sw.os',
			variants: [
				{
					version: 'wheezy',
				},
				{
					version: 'jessie',
				},
				{
					version: 'sid',
				},
			],
		});

		expect(contracts).to.deep.equal([
			new Contract({
				name: 'debian wheezy',
				slug: 'debian',
				version: 'wheezy',
				type: 'sw.os',
			}),
			new Contract({
				name: 'debian jessie',
				slug: 'debian',
				version: 'jessie',
				type: 'sw.os',
			}),
			new Contract({
				name: 'debian sid',
				slug: 'debian',
				version: 'sid',
				type: 'sw.os',
			}),
		]);
	});

	it('should build contracts with nested variants and templates', () => {
		const contracts = Contract.build({
			slug: 'fedora',
			type: 'sw.os',
			version: '1',
			data: {
				libc: 'glibc',
				latest: '37',
				versionList: '`37 (latest)`, `38`',
			},
			name: 'Fedora {{this.version}}',
			requires: [
				{ type: 'sw.blob', slug: 'balena-idle' },
				{ type: 'sw.blob', slug: 'balena-info' },
				{ type: 'sw.blob', slug: 'balena-xbuild' },
				{ type: 'sw.blob', slug: 'entry' },
			],
			assets: {
				test: {
					main: 'test-os',
					name: 'test-os.sh',
					commit: 'a95300eda2320833e537ca20d728a870bf02177d',
					url: 'https://raw.githubusercontent.com/balena-io-library/base-images/{{this.assets.test.commit}}/scripts/assets/tests/{{this.assets.test.name}}',
				},
			},
			variants: [
				{
					variants: [{ version: '37' }, { version: '38' }],
					requires: [
						{ type: 'sw.blob', slug: 'qemu' },
						{ type: 'arch.sw', slug: 'aarch64' },
					],
				},
				{
					variants: [{ version: '37' }, { version: '38' }],
					requires: [{ type: 'arch.sw', slug: 'amd64' }],
				},
			],
		});

		const data = {
			libc: 'glibc',
			latest: '37',
			versionList: '`37 (latest)`, `38`',
		};

		const assets = {
			test: {
				main: 'test-os',
				name: 'test-os.sh',
				commit: 'a95300eda2320833e537ca20d728a870bf02177d',
				url: 'https://raw.githubusercontent.com/balena-io-library/base-images/a95300eda2320833e537ca20d728a870bf02177d/scripts/assets/tests/test-os.sh',
			},
		};

		const baseRequires = [
			{ type: 'sw.blob', slug: 'balena-idle' },
			{ type: 'sw.blob', slug: 'balena-info' },
			{ type: 'sw.blob', slug: 'balena-xbuild' },
			{ type: 'sw.blob', slug: 'entry' },
		];

		expect(contracts).to.deep.equal([
			new Contract({
				slug: 'fedora',
				type: 'sw.os',
				version: '37',
				name: 'Fedora 37',
				data,
				requires: baseRequires.concat([
					{ type: 'sw.blob', slug: 'qemu' },
					{ type: 'arch.sw', slug: 'aarch64' },
				]),
				assets,
			}),
			new Contract({
				slug: 'fedora',
				type: 'sw.os',
				version: '38',
				name: 'Fedora 38',
				data,
				requires: baseRequires.concat([
					{ type: 'sw.blob', slug: 'qemu' },
					{ type: 'arch.sw', slug: 'aarch64' },
				]),
				assets,
			}),
			new Contract({
				slug: 'fedora',
				type: 'sw.os',
				version: '37',
				name: 'Fedora 37',
				data,
				requires: baseRequires.concat([{ type: 'arch.sw', slug: 'amd64' }]),
				assets,
			}),
			new Contract({
				slug: 'fedora',
				type: 'sw.os',
				version: '38',
				name: 'Fedora 38',
				data,
				requires: baseRequires.concat([{ type: 'arch.sw', slug: 'amd64' }]),
				assets,
			}),
		]);
	});

	it('should expand contract aliases', () => {
		const contracts = Contract.build({
			slug: 'debian',
			type: 'sw.os',
			version: 'jessie',
			aliases: ['foo', 'bar'],
		});

		expect(contracts).to.deep.equal([
			new Contract({
				slug: 'foo',
				version: 'jessie',
				type: 'sw.os',
				canonicalSlug: 'debian',
			}),
			new Contract({
				slug: 'bar',
				version: 'jessie',
				type: 'sw.os',
				canonicalSlug: 'debian',
			}),
			new Contract({
				slug: 'debian',
				version: 'jessie',
				type: 'sw.os',
			}),
		]);
	});

	it('should build contracts with variants and aliases', () => {
		const contracts = Contract.build({
			name: 'debian {{this.version}}',
			slug: 'debian',
			type: 'sw.os',
			variants: [
				{
					version: 'wheezy',
				},
				{
					version: 'jessie',
				},
			],
			aliases: ['foo', 'bar'],
		});

		expect(contracts).to.deep.equal([
			new Contract({
				name: 'debian wheezy',
				slug: 'foo',
				type: 'sw.os',
				version: 'wheezy',
				canonicalSlug: 'debian',
			}),
			new Contract({
				name: 'debian wheezy',
				slug: 'bar',
				type: 'sw.os',
				version: 'wheezy',
				canonicalSlug: 'debian',
			}),
			new Contract({
				name: 'debian wheezy',
				slug: 'debian',
				version: 'wheezy',
				type: 'sw.os',
			}),
			new Contract({
				name: 'debian jessie',
				slug: 'foo',
				type: 'sw.os',
				version: 'jessie',
				canonicalSlug: 'debian',
			}),
			new Contract({
				name: 'debian jessie',
				slug: 'bar',
				type: 'sw.os',
				version: 'jessie',
				canonicalSlug: 'debian',
			}),
			new Contract({
				name: 'debian jessie',
				slug: 'debian',
				version: 'jessie',
				type: 'sw.os',
			}),
		]);
	});
});
