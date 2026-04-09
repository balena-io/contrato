/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';
import { compileContract } from '../../lib/template';

describe('compileContract', () => {
	it('should compile a contract without templates', () => {
		expect(
			compileContract({
				type: 'distro',
				name: 'Debian',
				version: 'wheezy',
				slug: 'debian',
			}),
		).to.deep.equal({
			type: 'distro',
			name: 'Debian',
			version: 'wheezy',
			slug: 'debian',
		});
	});

	it('should compile a single top level template', () => {
		expect(
			compileContract({
				type: 'distro',
				name: 'Debian {{this.version}}',
				version: 'wheezy',
				slug: 'debian',
			}),
		).to.deep.equal({
			type: 'distro',
			name: 'Debian wheezy',
			version: 'wheezy',
			slug: 'debian',
		});
	});

	it('should compile templates inside arrays', () => {
		expect(
			compileContract({
				type: 'distro',
				name: 'Debian',
				slug: 'debian',
				random: ['{{this.name}}', '{{this.name}}', '{{this.name}}'],
				requires: [
					{
						name: '{{this.name}} ({{this.type}})',
					},
				],
			}),
		).to.deep.equal({
			type: 'distro',
			name: 'Debian',
			slug: 'debian',
			random: ['Debian', 'Debian', 'Debian'],
			requires: [
				{
					name: 'Debian (distro)',
				},
			],
		});
	});

	it('should compile multiple top level templates', () => {
		expect(
			compileContract({
				type: 'distro',
				name: 'Debian {{this.version}}',
				version: 'wheezy',
				slug: 'debian-{{this.version}}',
			}),
		).to.deep.equal({
			type: 'distro',
			name: 'Debian wheezy',
			version: 'wheezy',
			slug: 'debian-wheezy',
		});
	});

	it('should compile a single nested template', () => {
		expect(
			compileContract({
				type: 'distro',
				name: 'Debian',
				version: 'wheezy',
				slug: 'debian',
				data: {
					foo: {
						bar: {
							baz: '{{this.type}}',
						},
					},
				},
			}),
		).to.deep.equal({
			type: 'distro',
			name: 'Debian',
			version: 'wheezy',
			slug: 'debian',
			data: {
				foo: {
					bar: {
						baz: 'distro',
					},
				},
			},
		});
	});

	it('should leave missing values as interpolations', () => {
		expect(
			compileContract({
				type: 'distro',
				name: 'Debian',
				version: '{{this.data.distroName}}',
				slug: 'debian',
			}),
		).to.deep.equal({
			type: 'distro',
			name: 'Debian',
			version: '{{this.data.distroName}}',
			slug: 'debian',
		});
	});

	it('should be able to blacklist a top level element', () => {
		const result = compileContract(
			{
				type: 'distro',
				version: '7',
				name: 'Debian v{{this.version}}',
				data: {
					name: 'debian',
				},
				slug: '{{this.data.name}}',
			},
			{
				blacklist: new Set(['name']),
			},
		);

		expect(result).to.deep.equal({
			type: 'distro',
			version: '7',
			name: 'Debian v{{this.version}}',
			data: {
				name: 'debian',
			},
			slug: 'debian',
		});
	});

	it('should be able to blacklist a nested element', () => {
		const result = compileContract(
			{
				type: 'distro',
				version: '7',
				name: 'Debian v{{this.version}}',
				data: {
					name: 'debian',
					foo: {
						type: '{{this.type}}',
					},
				},
				slug: '{{this.data.name}}',
			},
			{
				blacklist: new Set(['data.foo.type']),
			},
		);

		expect(result).to.deep.equal({
			type: 'distro',
			version: '7',
			name: 'Debian v7',
			data: {
				name: 'debian',
				foo: {
					type: '{{this.type}}',
				},
			},
			slug: 'debian',
		});
	});

	it('should be able to blacklist more than one element', () => {
		const result = compileContract(
			{
				type: 'distro',
				version: '7',
				name: 'Debian v{{this.version}}',
				data: {
					name: 'debian',
					foo: {
						type: '{{this.type}}',
					},
				},
				slug: '{{this.data.name}}',
			},
			{
				blacklist: new Set(['data.foo.type', 'name']),
			},
		);

		expect(result).to.deep.equal({
			type: 'distro',
			version: '7',
			name: 'Debian v{{this.version}}',
			data: {
				name: 'debian',
				foo: {
					type: '{{this.type}}',
				},
			},
			slug: 'debian',
		});
	});

	it('should be able to blacklist elements inside arrays', () => {
		const result = compileContract(
			{
				slug: 'debian',
				type: 'distro',
				random: ['{{this.slug}}', '{{this.slug}}', '{{this.slug}}'],
			},
			{
				blacklist: new Set(['random.1']),
			},
		);

		expect(result).to.deep.equal({
			slug: 'debian',
			type: 'distro',
			random: ['debian', '{{this.slug}}', 'debian'],
		});
	});

	it('should be able to blacklist a whole subtree', () => {
		const result = compileContract(
			{
				type: 'distro',
				version: '7',
				name: 'Debian v{{this.version}}',
				data: {
					name: 'debian',
					foo: {
						type: '{{this.type}}',
					},
				},
				slug: '{{this.data.name}}',
			},
			{
				blacklist: new Set(['data']),
			},
		);

		expect(result).to.deep.equal({
			type: 'distro',
			version: '7',
			name: 'Debian v7',
			data: {
				name: 'debian',
				foo: {
					type: '{{this.type}}',
				},
			},
			slug: 'debian',
		});
	});

	it('should stringify object when referenced value is an object', () => {
		const result = compileContract({
			type: 'distro',
			name: '{{this.data}}',
			slug: 'debian',
			data: {
				arch: 'amd64',
			},
		});

		// JS String.replace calls .toString() on objects, producing [object Object].
		// The Rust implementation uses JSON.stringify instead, but the key invariant
		// is that the result remains a string, not a subtree.
		expect(result).to.deep.equal({
			type: 'distro',
			name: '[object Object]',
			slug: 'debian',
			data: {
				arch: 'amd64',
			},
		});
	});
});
