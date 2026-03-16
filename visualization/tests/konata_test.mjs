/**
 * Konata Renderer Automated Test
 *
 * Tests the following features:
 * 1. Store instruction display format
 * 2. Memory address tooltip on hover
 * 3. Double-click to align timeline
 * 4. Search to align timeline
 */

import { chromium } from 'playwright';
import fs from 'fs';
import path from 'path';

const TEST_URL = 'http://localhost:8080/test_konata.html';
const SCREENSHOT_DIR = './test_screenshots';

// Ensure screenshot directory exists
if (!fs.existsSync(SCREENSHOT_DIR)) {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
}

async function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function runTests() {
    console.log('🚀 Starting Konata Renderer Automated Tests...\n');

    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext({
        viewport: { width: 1200, height: 800 }
    });
    const page = await context.newPage();

    // Capture console logs
    const consoleLogs = [];
    page.on('console', msg => {
        const logEntry = `[${msg.type()}] ${msg.text()}`;
        consoleLogs.push(logEntry);
        console.log(`📋 Console: ${logEntry}`);
    });

    // Capture errors
    const errors = [];
    page.on('pageerror', error => {
        errors.push(error.toString());
        console.log(`❌ Error: ${error}`);
    });

    try {
        // ============================================
        // Test 1: Page Load
        // ============================================
        console.log('\n📌 Test 1: Page Load');
        console.log('─'.repeat(50));

        await page.goto(TEST_URL, { waitUntil: 'networkidle' });
        await sleep(500);

        // Take initial screenshot
        await page.screenshot({ path: `${SCREENSHOT_DIR}/01_initial.png` });
        console.log('✅ Page loaded successfully');
        console.log('   Screenshot saved: 01_initial.png');

        // Check if renderer was created
        const rendererCreated = await page.evaluate(() => {
            return typeof window.renderer !== 'undefined';
        });
        console.log(`   Renderer created: ${rendererCreated ? '✅' : '❌'}`);

        // ============================================
        // Test 2: Load Test Data
        // ============================================
        console.log('\n📌 Test 2: Load Test Data');
        console.log('─'.repeat(50));

        await page.click('button:has-text("Load Test Data")');
        await sleep(1000);

        await page.screenshot({ path: `${SCREENSHOT_DIR}/02_data_loaded.png` });
        console.log('✅ Test data loaded');
        console.log('   Screenshot saved: 02_data_loaded.png');

        // Check if data was loaded
        const dataLoaded = await page.evaluate(() => {
            if (!window.renderer) return false;
            return window.renderer.ops && window.renderer.ops.length > 0;
        });
        console.log(`   Operations loaded: ${dataLoaded ? '✅' : '❌'}`);

        if (dataLoaded) {
            const opCount = await page.evaluate(() => window.renderer.ops.length);
            console.log(`   Total operations: ${opCount}`);
        }

        // ============================================
        // Test 3: Store Instruction Display
        // ============================================
        console.log('\n📌 Test 3: Store Instruction Display');
        console.log('─'.repeat(50));

        // Find a STORE instruction (every 5th instruction starting at index 4)
        const storeInfo = await page.evaluate(() => {
            if (!window.renderer || !window.renderer.ops) return null;

            // Find STORE instructions
            const storeOps = window.renderer.ops.filter(op =>
                op.labelName && op.labelName.includes('STORE')
            );

            if (storeOps.length === 0) return null;

            const op = storeOps[0];
            return {
                id: op.id,
                labelName: op.labelName,
                isMemory: op.isMemory,
                memAddr: op.memAddr,
                srcRegs: op.srcRegs,
                dstRegs: op.dstRegs,
                formatMemAddr: op.formatMemAddr ? op.formatMemAddr() : null
            };
        });

        if (storeInfo) {
            console.log(`   Found STORE instruction: ID=${storeInfo.id}`);
            console.log(`   Label: ${storeInfo.labelName}`);
            console.log(`   isMemory: ${storeInfo.isMemory}`);
            console.log(`   memAddr: ${storeInfo.memAddr} (${storeInfo.formatMemAddr})`);
            console.log(`   srcRegs: ${JSON.stringify(storeInfo.srcRegs)}`);
            console.log(`   dstRegs: ${JSON.stringify(storeInfo.dstRegs)}`);
            console.log(`   Memory detection: ${storeInfo.isMemory === true ? '✅' : '❌'}`);
        } else {
            console.log('   ❌ No STORE instruction found');
        }

        // ============================================
        // Test 4: Hover and Tooltip
        // ============================================
        console.log('\n📌 Test 4: Hover and Tooltip');
        console.log('─'.repeat(50));

        // Get canvas position
        const canvasBounds = await page.locator('#container canvas').boundingBox();

        if (canvasBounds) {
            // Hover over a STORE instruction (approximately at row 4)
            const hoverY = canvasBounds.y + 70 + 4 * 26; // header + timeline + 4 rows
            const hoverX = canvasBounds.x + 100; // Left panel area

            await page.mouse.move(hoverX, hoverY);
            await sleep(300);

            await page.screenshot({ path: `${SCREENSHOT_DIR}/03_hover_store.png` });
            console.log('✅ Hovered over STORE instruction');
            console.log('   Screenshot saved: 03_hover_store.png');

            // Check if tooltip is visible
            const tooltipVisible = await page.evaluate(() => {
                const tooltip = document.querySelector('.konata-tooltip');
                if (!tooltip) return { visible: false, exists: false };
                return {
                    exists: true,
                    visible: tooltip.style.display !== 'none',
                    text: tooltip.textContent,
                    display: tooltip.style.display
                };
            });

            console.log(`   Tooltip exists: ${tooltipVisible.exists ? '✅' : '❌'}`);
            console.log(`   Tooltip visible: ${tooltipVisible.visible ? '✅' : '❌'}`);
            if (tooltipVisible.visible) {
                console.log(`   Tooltip text: "${tooltipVisible.text}"`);
            }
        }

        // ============================================
        // Test 5: Double-Click to Align Timeline
        // ============================================
        console.log('\n📌 Test 5: Double-Click to Align Timeline');
        console.log('─'.repeat(50));

        if (canvasBounds) {
            // Get initial scroll position
            const initialScroll = await page.evaluate(() => {
                return {
                    scrollX: window.renderer ? window.renderer.scrollX : 0,
                    scrollY: window.renderer ? window.renderer.scrollY : 0,
                    cycleOffset: window.renderer ? window.renderer.cycleOffset : 0
                };
            });
            console.log(`   Initial scroll: X=${initialScroll.scrollX}, Y=${initialScroll.scrollY}`);
            console.log(`   Initial cycleOffset: ${initialScroll.cycleOffset}`);

            // Scroll right first to change the view
            await page.mouse.wheel(500, 0);
            await sleep(300);

            const afterWheelScroll = await page.evaluate(() => {
                return {
                    scrollX: window.renderer ? window.renderer.scrollX : 0,
                    cycleOffset: window.renderer ? window.renderer.cycleOffset : 0
                };
            });
            console.log(`   After wheel: scrollX=${afterWheelScroll.scrollX}, cycleOffset=${afterWheelScroll.cycleOffset}`);

            // Double-click on instruction row 5
            const clickY = canvasBounds.y + 70 + 5 * 26;
            const clickX = canvasBounds.x + 100;

            await page.mouse.dblclick(clickX, clickY);
            await sleep(500);

            await page.screenshot({ path: `${SCREENSHOT_DIR}/04_double_click.png` });
            console.log('✅ Double-clicked on instruction');
            console.log('   Screenshot saved: 04_double_click.png');

            // Check if scroll changed (timeline aligned)
            const afterDoubleClick = await page.evaluate(() => {
                const r = window.renderer;
                const selectedOp = r ? r.selectedOp : null;
                return {
                    scrollX: r ? r.scrollX : 0,
                    scrollY: r ? r.scrollY : 0,
                    cycleOffset: r ? r.cycleOffset : 0,
                    selectedOpId: selectedOp ? selectedOp.id : null,
                    scrollToOpExists: r ? typeof r.scrollToOp === 'function' : false
                };
            });

            console.log(`   After double-click: scrollX=${afterDoubleClick.scrollX}, scrollY=${afterDoubleClick.scrollY}`);
            console.log(`   cycleOffset: ${afterDoubleClick.cycleOffset}`);
            console.log(`   Selected op ID: ${afterDoubleClick.selectedOpId}`);
            console.log(`   scrollToOp method exists: ${afterDoubleClick.scrollToOpExists ? '✅' : '❌'}`);
        }

        // ============================================
        // Test 6: Search Functionality
        // ============================================
        console.log('\n📌 Test 6: Search Functionality');
        console.log('─'.repeat(50));

        // Test search by calling the search method
        const searchResult = await page.evaluate(() => {
            if (!window.renderer) return { success: false, error: 'No renderer' };

            try {
                const count = window.renderer.search('5');
                return {
                    success: true,
                    resultCount: count,
                    searchResults: window.renderer.searchResults,
                    selectedOpId: window.renderer.selectedOp ? window.renderer.selectedOp.id : null
                };
            } catch (e) {
                return { success: false, error: e.message };
            }
        });

        console.log(`   Search for '5': ${searchResult.success ? '✅' : '❌'}`);
        if (searchResult.success) {
            console.log(`   Results found: ${searchResult.resultCount}`);
            console.log(`   Selected op ID: ${searchResult.selectedOpId}`);
        }

        await sleep(500);
        await page.screenshot({ path: `${SCREENSHOT_DIR}/05_search.png` });
        console.log('   Screenshot saved: 05_search.png');

        // ============================================
        // Test 7: Zoom Controls
        // ============================================
        console.log('\n📌 Test 7: Zoom Controls');
        console.log('─'.repeat(50));

        const zoomTest = await page.evaluate(() => {
            if (!window.renderer) return { success: false };

            const initialZoom = window.renderer.zoom;
            window.renderer.zoomIn();
            const afterZoomIn = window.renderer.zoom;
            window.renderer.zoomOut();
            const afterZoomOut = window.renderer.zoom;

            return {
                success: true,
                initialZoom,
                afterZoomIn,
                afterZoomOut
            };
        });

        if (zoomTest.success) {
            console.log(`   Initial zoom: ${zoomTest.initialZoom}`);
            console.log(`   After zoom in: ${zoomTest.afterZoomIn}`);
            console.log(`   After zoom out: ${zoomTest.afterZoomOut}`);
            console.log(`   Zoom in works: ${zoomTest.afterZoomIn > zoomTest.initialZoom ? '✅' : '❌'}`);
        }

        // ============================================
        // Test Summary
        // ============================================
        console.log('\n' + '='.repeat(50));
        console.log('📊 TEST SUMMARY');
        console.log('='.repeat(50));

        console.log('\n📷 Screenshots saved:');
        const screenshots = fs.readdirSync(SCREENSHOT_DIR).filter(f => f.endsWith('.png'));
        screenshots.forEach(s => console.log(`   - ${SCREENSHOT_DIR}/${s}`));

        console.log('\n📋 Console Logs:');
        consoleLogs.slice(0, 20).forEach(log => console.log(`   ${log}`));
        if (consoleLogs.length > 20) {
            console.log(`   ... and ${consoleLogs.length - 20} more logs`);
        }

        if (errors.length > 0) {
            console.log('\n❌ Errors Found:');
            errors.forEach(e => console.log(`   ${e}`));
        } else {
            console.log('\n✅ No JavaScript errors detected');
        }

        // Save console logs to file
        fs.writeFileSync(`${SCREENSHOT_DIR}/console_logs.txt`, consoleLogs.join('\n'));
        console.log(`\n📝 Console logs saved to: ${SCREENSHOT_DIR}/console_logs.txt`);

        // Save test results
        const testResults = {
            timestamp: new Date().toISOString(),
            consoleLogs,
            errors,
            screenshots
        };
        fs.writeFileSync(`${SCREENSHOT_DIR}/test_results.json`, JSON.stringify(testResults, null, 2));
        console.log(`📝 Test results saved to: ${SCREENSHOT_DIR}/test_results.json`);

    } catch (error) {
        console.error('❌ Test failed with error:', error);
    } finally {
        await browser.close();
    }

    console.log('\n✅ Tests completed!');
}

runTests().catch(console.error);
