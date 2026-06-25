/*
 * Copyright (C) Balena.io - All Rights Reserved
 * Unauthorized copying of this file, via any medium is strictly prohibited.
 * Proprietary and confidential.
 */

import { expect } from '../chai';
import * as _ from 'lodash';

import Contract from '../../lib/contract';

import Blueprint from '../../lib/blueprint';
import { parse } from '../../lib/cardinality';

describe('Blueprint constructor', () => {
	it('should be a contract', () => {
		const blueprint = new Blueprint({
			'hw.device-type': 1,
		});

		expect(blueprint instanceof Contract).to.be.true;
	});

	it('should set a proper type', () => {
		const blueprint = new Blueprint({
			'hw.device-type': 1,
		});

		expect(blueprint.raw.type).to.equal('meta.blueprint');
	});

	it('should be a hashable contract', () => {
		const blueprint = new Blueprint({
			'hw.device-type': 1,
		});

		expect(typeof blueprint.hash()).to.equal('string');
	});

	it('should parse a layout with one number selector', () => {
		const blueprint = new Blueprint({
			'hw.device-type': 1,
		});

		expect(blueprint.metadata.layout).to.deep.equal({
			types: new Set(['hw.device-type']),
			finite: {
				selectors: {
					'hw.device-type': [
						{
							cardinality: parse([1, 1]),
							filter: undefined,
							version: undefined,
							type: 'hw.device-type',
						},
					],
				},
				types: new Set(['hw.device-type']),
			},
			infinite: {
				selectors: {},
				types: new Set(),
			},
		});
	});

	it('should parse a layout with one finite and one infinite selectors', () => {
		const blueprint = new Blueprint({
			'hw.device-type': 2,
			'arch.sw': '1+',
		});

		expect(blueprint.metadata.layout).to.deep.equal({
			types: new Set(['hw.device-type', 'arch.sw']),
			finite: {
				selectors: {
					'hw.device-type': [
						{
							cardinality: parse([2, 2]),
							filter: undefined,
							version: undefined,
							type: 'hw.device-type',
						},
					],
				},
				types: new Set(['hw.device-type']),
			},
			infinite: {
				selectors: {
					'arch.sw': [
						{
							cardinality: parse([1, Infinity]),
							filter: undefined,
							version: undefined,
							type: 'arch.sw',
						},
					],
				},
				types: new Set(['arch.sw']),
			},
		});
	});

	it('should support object layout selectors', () => {
		const filterFunction = _.identity;

		const blueprint = new Blueprint({
			'hw.device-type': {
				cardinality: [2, 2],
			},
			'arch.sw': {
				cardinality: '1+',
				filter: filterFunction,
			},
		});

		expect(blueprint.metadata.layout).to.deep.equal({
			types: new Set(['hw.device-type', 'arch.sw']),
			finite: {
				selectors: {
					'hw.device-type': [
						{
							cardinality: parse([2, 2]),
							filter: undefined,
							version: undefined,
							type: 'hw.device-type',
						},
					],
				},
				types: new Set(['hw.device-type']),
			},
			infinite: {
				selectors: {
					'arch.sw': [
						{
							cardinality: parse([1, Infinity]),
							filter: filterFunction,
							version: undefined,
							type: 'arch.sw',
						},
					],
				},
				types: new Set(['arch.sw']),
			},
		});
	});

	it('should allow passing a skeleton object', () => {
		const blueprint = new Blueprint(
			{
				'hw.device-type': 1,
			},
			{
				type: 'sw.os-image',
				name: 'Generic OS Image',
			},
		);

		expect(blueprint.raw.skeleton).to.deep.equal({
			type: 'sw.os-image',
			name: 'Generic OS Image',
		});
	});
});
