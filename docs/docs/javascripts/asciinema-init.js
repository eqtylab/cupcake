/**
 * Asciinema Player Initialization for Zensical
 * 
 * Automatically initializes asciinema players on page load.
 * Uses the document$ observable for compatibility with instant navigation.
 * 
 * Usage in markdown:
 *   <div class="cast-player" data-cast="path/to/recording.cast"></div>
 * 
 * Optional data attributes:
 *   data-cast      - Path to .cast file (required)
 *   data-poster    - Poster frame: "npt:1:30" for time, "data:text/plain,..." for text
 *   data-autoplay  - Auto-play on load: "true" or "false" (default: false)
 *   data-loop      - Loop playback: "true" or "false" (default: false)
 *   data-speed     - Playback speed: number (default: 1)
 *   data-rows      - Override terminal rows
 *   data-cols      - Override terminal columns
 *   data-fit       - Fit mode: "width", "height", "both", "none" (default: "width")
 */

(function() {
  'use strict';

  /**
   * Initialize a single player element
   */
  function initPlayer(element) {
    // Check if AsciinemaPlayer is available
    if (typeof AsciinemaPlayer === 'undefined') {
      console.error('AsciinemaPlayer is not loaded');
      return null;
    }

    var castUrl = element.getAttribute('data-cast');
    if (!castUrl) {
      console.warn('Cast player missing data-cast attribute:', element);
      return null;
    }

    // Skip if already initialized
    if (element.hasAttribute('data-initialized')) {
      return null;
    }

    // Build options from data attributes
    // Use 'zensical' theme which adapts to light/dark via CSS
    var options = {
      theme: 'zensical',
      fit: element.getAttribute('data-fit') || 'width',
      idleTimeLimit: 2,
      preload: true
    };

    // Optional attributes
    if (element.hasAttribute('data-poster')) {
      options.poster = element.getAttribute('data-poster');
    }
    if (element.hasAttribute('data-autoplay')) {
      options.autoPlay = element.getAttribute('data-autoplay') === 'true';
    }
    if (element.hasAttribute('data-loop')) {
      options.loop = element.getAttribute('data-loop') === 'true';
    }
    if (element.hasAttribute('data-speed')) {
      options.speed = parseFloat(element.getAttribute('data-speed'));
    }
    if (element.hasAttribute('data-rows')) {
      options.rows = parseInt(element.getAttribute('data-rows'), 10);
    }
    if (element.hasAttribute('data-cols')) {
      options.cols = parseInt(element.getAttribute('data-cols'), 10);
    }

    // Create the player
    try {
      var player = AsciinemaPlayer.create(castUrl, element, options);
      element.setAttribute('data-initialized', 'true');
      return player;
    } catch (err) {
      console.error('Failed to create asciinema player:', err);
      return null;
    }
  }

  /**
   * Initialize all players on the page
   */
  function initAllPlayers() {
    var containers = document.querySelectorAll('.cast-player:not([data-initialized])');
    
    if (containers.length === 0) {
      return;
    }

    // Check if AsciinemaPlayer is available
    if (typeof AsciinemaPlayer === 'undefined') {
      console.warn('AsciinemaPlayer not available, retrying in 100ms...');
      setTimeout(initAllPlayers, 100);
      return;
    }

    containers.forEach(function(container) {
      initPlayer(container);
    });
  }

  /**
   * Main initialization function
   */
  function init() {
    // Try to initialize immediately
    initAllPlayers();
  }

  // Initialize using multiple strategies to ensure it works

  // Strategy 1: Zensical's document$ observable (for instant navigation)
  if (typeof document$ !== 'undefined') {
    document$.subscribe(function() {
      init();
    });
  }

  // Strategy 2: DOMContentLoaded (standard approach)
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    // DOM already loaded, init immediately
    init();
  }

  // Strategy 3: Window load event (fallback for slow script loading)
  window.addEventListener('load', init);

})();
