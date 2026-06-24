/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';

import Contract from '../../lib/contract';

describe('Contract createMatcher', () => {
	it('should create contract instance', () => {
		const matcher = Contract.createMatcher({
			type: 'arch.sw',
			slug: 'armv7hf',
		});

		expect(matcher instanceof Contract).to.be.true;
	});

	it('should include the properties in data', () => {
		const matcher = Contract.createMatcher({
			type: 'arch.sw',
			slug: 'armv7hf',
		});

		expect(matcher.raw().data).to.deep.equal({
			type: 'arch.sw',
			slug: 'armv7hf',
		});
	});

	it('should set the type appropriately', () => {
		const matcher = Contract.createMatcher({
			type: 'arch.sw',
			slug: 'armv7hf',
		});

		expect(matcher.getType()).to.equal('meta.matcher');
	});

	it('should be able to set the operation name', () => {
		const matcher = Contract.createMatcher(
			{
				type: 'arch.sw',
				slug: 'armv7hf',
			},
			{
				operation: 'or',
			},
		);

		expect(matcher.raw().operation).to.equal('or');
	});

	it('should reject a matcher with properties outside the allowed fields', () => {
		expect(() =>
			Contract.createMatcher({
				type: 'arch.sw',
				slug: 'armv7hf',
				arch: 'armv7hf',
			} as any),
		).to.throw(
			'unknown field `arch`, expected one of `type`, `slug`, `version`, `data`',
		);
	});

	it('should reject a sub-matcher with properties outside the allowed fields', () => {
		expect(() =>
			Contract.createMatcher(
				[
					{ type: 'arch.sw', slug: 'armv7hf' },
					{ type: 'arch.sw', arch: 'amd64' } as any,
				],
				{ operation: 'or' },
			),
		).to.throw(
			'unknown field `arch`, expected one of `type`, `slug`, `version`, `data`',
		);
	});
});
