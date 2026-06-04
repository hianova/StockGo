export function renderAdBanner(): string {
  // This uses a fixed-height container to avoid CLS (Cumulative Layout Shift)
  return `
    <div class="ad-container">
      <div class="ad-content">
        <span class="ad-label">Sponsored</span>
        <h3>Open a Brokerage Account Today!</h3>
        <p>Get zero-commission trades and real-time market data.</p>
        <a href="#" class="ad-btn">Learn More</a>
      </div>
    </div>
  `;
}
