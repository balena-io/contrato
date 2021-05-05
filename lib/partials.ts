/*
 * Copyright 2017 resin.io
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

'use strict'

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable '_'.
const _ = require('lodash')

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable 'Contract'.
const Contract = require('./contract')

// @ts-expect-error ts-migrate(2580) FIXME: Cannot find name 'require'. Do you need to install... Remove this comment to see the full error message
const path = require('path')

// @ts-expect-error ts-migrate(2580) FIXME: Cannot find name 'require'. Do you need to install... Remove this comment to see the full error message
const fs = require('fs')

// @ts-expect-error ts-migrate(2580) FIXME: Cannot find name 'require'. Do you need to install... Remove this comment to see the full error message
const debug = require('debug')('partials')

// @ts-expect-error ts-migrate(2580) FIXME: Cannot find name 'require'. Do you need to install... Remove this comment to see the full error message
const handlebars = require('handlebars')

// @ts-expect-error ts-migrate(2451) FIXME: Cannot redeclare block-scoped variable 'utils'.
const utils = require('./utils')

/**
 * @summary Delimiter to use between contract references
 * @type {String}
 * @private
 */
const REFERENCE_DELIMITER = '+'

/**
 * @summary Calculate the paths to search for a partial given a contract
 * @function
 * @private
 *
 * @param {String} name - partial name (without extension)
 * @param {Object} context - context contract
 * @param {Object} options - options
 * @param {String} options.baseDirectory - partials directory
 * @param {String[]} options.structure - the type hierarchy of the partials directory
 *
 * @returns {String[]} possible partial paths
 *
 * @example
 * const context = new Contract({ ... })
 * context.addChildren([ ... ])
 *
 * const paths = partials.findPartial('my-partial', context, {
 *   baseDirectory: 'my/partials',
 *   // The base directory hierarchy is <distro>/<stack>
 *   structure: [ 'sw.os', 'sw.stack' ]
 * })
 *
 * paths.forEach((path) => {
 *   console.log(`Trying to load ${path}...`)
 * })
 */
// @ts-expect-error ts-migrate(2304) FIXME: Cannot find name 'exports'.
exports.findPartial = (name, context, options) => {
  return _.chain(options.structure)
    .map((type) => {
      const children = context.getChildrenByType(type)
      const contracts = _.chain(children).map((contract) => {
        // We need to replace the alias slug with canonical slug when finding partial
        // since the aliases will use canonical slug to avoid duplication.
        const rawContract = contract.toJSON()
        rawContract.slug = contract.getCanonicalSlug()
        return new Contract(rawContract)
      })
        .sortBy((contract) => {
          return contract.getSlug()
        })
        .value()

      return [
        _.join(_.invokeMap(contracts, 'getReferenceString'), REFERENCE_DELIMITER),
        _.join(_.invokeMap(contracts, 'getSlug'), REFERENCE_DELIMITER)
      ]
    })
    .thru((combinations) => {
      const products = utils.cartesianProductWith(combinations, (accumulator, element) => {
        return accumulator.concat([ element ])
      })

      const slices = _.reduce(_.range(options.structure.length, 1, -1), (accumulator, slice) => {
        return accumulator.concat(_.invokeMap(products, 'slice', 0, slice))
      }, [])

      const fallbackPaths = _.chain(combinations)

        // @ts-expect-error ts-migrate(6133) FIXME: 'value' is declared but its value is never read.
        .reduce((accumulator, value, index, collection) => {
          return _.map([
            _.map(collection, _.first),
            _.map(collection, _.last)
          ], (list) => {
            return _.take(list, index + 1)
          }).concat(accumulator)
        }, [])
        .value()

      return products.concat(slices).concat(fallbackPaths)
    })
    .map((references) => {
      return [ _.join(references, REFERENCE_DELIMITER), name ]
    })
    .concat([ [ name ] ])
    .map((paths) => {
      const absolutePath = [ options.baseDirectory ].concat(paths)
      return `${path.join(...absolutePath)}.tpl`
    })
    .uniq()
    .value()
}

handlebars.registerHelper('import', (options) => {
  const settings = options.data.root.settings

  // @ts-expect-error ts-migrate(2304) FIXME: Cannot find name 'exports'.
  const partialPaths = exports.findPartial(options.hash.partial, settings.context, {
    baseDirectory: path.join(settings.directory, options.hash.combination),
    structure: _.map(_.split(options.hash.combination, REFERENCE_DELIMITER), _.trim)
  })

  for (const partialPath of partialPaths) {
    const partialContent = _.attempt(fs.readFileSync, partialPath, {
      encoding: 'utf8'
    })

    if (_.isError(partialContent)) {
      if (partialContent.code === 'ENOENT') {
        debug(`Ignoring ${partialPath}`)
        continue
      }

      throw partialContent
    }

    debug(`Using ${partialPath}`)

    // We need to prevent handlebars from encoding the string as HTML,
    // and then we need to parse and recurse through the imported partials,
    // in case they have any interpolation that needs to be resolved.
    const safeContent = new handlebars.SafeString(partialContent.slice(0, partialContent.length - 1))
    const builtContent = handlebars.compile(safeContent.toString())(options.data.root)
    return new handlebars.SafeString(builtContent)
  }

  throw new Error(`Partial not found: ${options.hash.partial}`)
})

/**
* @example
*
* {{#if (eq hw.device-type.data.media.installation "sdcard")}}
*   SD card
* {{/if}}
* {{#if (eq hw.device-type.data.media.installation "usbkey")}}
*  USB drive
* {{/if}}
*
* {{#if (and hw.device-type.connectivity.wifi  hw.device-type.connectivity.ethernet)}}
*   Choose network interface
* {{/if}}
*/
handlebars.registerHelper({
  eq: (v1, v2) => { return v1 === v2 },
  ne: (v1, v2) => { return v1 !== v2 },
  lt: (v1, v2) => { return v1 < v2 },
  gt: (v1, v2) => { return v1 > v2 },
  lte: (v1, v2) => { return v1 <= v2 },
  gte: (v1, v2) => { return v1 >= v2 },
  and (...rest) {
    return rest.every(Boolean)
  },
  or (...rest) {
    return rest.slice(0, -1).some(Boolean)
  }
})

/**
 * @summary Build a template using a context contract
 * @function
 * @public
 * @memberof module:contrato
 * @name module:contrato.buildTemplate
 *
 * @param {String} template - template
 * @param {Object} context - context contract
 * @param {Object} options - options
 * @param {String} options.directory - partials directory
 * @returns {String} built template
 *
 * @example
 * const template = '....'
 * const context = new Contract({ ... })
 * context.addChildren([ ... ])
 *
 * const result = contrato.buildTemplate(template, context, {
 *   directory: './partials'
 * })
 *
 * console.log(result)
 */
// @ts-expect-error ts-migrate(2304) FIXME: Cannot find name 'exports'.
exports.buildTemplate = (template, context, options) => {
  const data = _.merge({
    settings: {
      directory: options.directory,
      context
    }
  }, context.toJSON().children)

  return utils.stripExtraBlankLines(handlebars.compile(template)(data))
}
