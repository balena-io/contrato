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

		expect(contracts.map((c) => c.raw)).to.deep.equal(
			[
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
			].map((c) => c.raw),
		);
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

		expect(contracts.map((c) => c.raw)).to.deep.equal(
			[
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
			].map((c) => c.raw),
		);
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

		expect(contracts.map((c) => c.raw)).to.deep.equal(
			[
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
			].map((c) => c.raw),
		);
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

		expect(contracts.map((c) => c.raw)).to.deep.equal(
			[
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
			].map((c) => c.raw),
		);
	});

	it('should expand contract aliases', () => {
		const contracts = Contract.build({
			slug: 'debian',
			type: 'sw.os',
			version: 'jessie',
			aliases: ['foo', 'bar'],
		});

		expect(contracts.map((c) => c.raw)).to.deep.equal(
			[
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
			].map((c) => c.raw),
		);
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

		expect(contracts.map((c) => c.raw)).to.deep.equal(
			[
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
			].map((c) => c.raw),
		);
	});
});
