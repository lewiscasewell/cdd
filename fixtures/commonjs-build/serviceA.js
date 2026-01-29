"use strict";
// CommonJS cycle: serviceA -> serviceB -> serviceA
const serviceB = require('./serviceB');

function doSomething() {
  return serviceB.helper();
}

module.exports = { doSomething };
