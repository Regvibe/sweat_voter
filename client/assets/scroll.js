const container = document.getElementById('egui-container');

// Allow the eGUI to handle scrolling
container.addEventListener('wheel', (event) => {
    container.scrollBy({
        top: event.deltaY, // Scroll vertically based on wheel movement
        behavior: 'smooth' // Optional smooth scrolling
    });
    event.preventDefault(); // Prevent default scrolling behavior
});
