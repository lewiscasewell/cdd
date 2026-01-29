"use strict";
// No cycle - standalone utility

function formatDate(date) {
  return date.toISOString();
}

module.exports = { formatDate };
