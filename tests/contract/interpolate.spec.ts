/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract interpolate', () => {
	it('should return the contract instance', () => {
		const contract = new Contract({
			name: 'Debian {{this.data.codename}}',
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			data: {
				url: 'https://contracts.org/downloads/{{this.type}}/{{this.slug}}/{{this.version}}.tar.gz',
			},
		});

		expect(contract.interpolate()).to.deep.equal(contract);
	});

	it('should not perform interpolation on children', () => {
		const contract = new Contract({
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			children: {
				foo: {
					bar: {
						slug: '{{this.version}}-child',
						type: 'foo.bar',
					},
				},
			},
		});

		expect(contract.interpolate().raw()).to.deep.equal({
			slug: 'debian',
			version: 'wheezy',
			type: 'sw.os',
			children: {
				foo: {
					bar: {
						slug: '{{this.version}}-child',
						type: 'foo.bar',
					},
				},
			},
		});
	});

	it('should throw and leave the contract untouched if the result is invalid', () => {
		// `{{this.children.*}}` only resolves once the children are in place.
		const contract = new Contract({
			type: 'sw.os',
			slug: '{{this.children.hw.device-type.name}}',
		});

		contract.addChild(
			new Contract({
				type: 'hw.device-type',
				slug: 'raspberrypi4',
				name: 'Raspberry Pi 4',
			}),
		);

		const before = contract.raw();
		const hash = contract.hash();

		expect(() => contract.interpolate()).to.throw('invalid slug');

		expect(contract.raw()).to.deep.equal(before);
		expect(contract.hash()).to.equal(hash);
	});
});
